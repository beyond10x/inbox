//! The listener: a loopback socket, one thread per connection, the expiry task, and stopping.

use std::io::{self, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use inbox::server::http;

use crate::config::Config;
use crate::expiry::sweep;
use crate::service::{Backend, Shared, answer};
use crate::signals::Signals;

/// The shortest pause between two sweeps, whatever `sweep_interval` says.
const MIN_SWEEP_INTERVAL: Duration = Duration::from_millis(1);

/// A service bound to its address and not yet serving.
#[derive(Debug)]
pub struct Server<B> {
    listener: TcpListener,
    address: SocketAddr,
    shared: Arc<Shared<B>>,
}

/// Tells a service to stop. Cloned freely, and sent to another thread.
#[derive(Debug, Clone)]
pub struct Stopper {
    signals: Arc<Signals>,
    address: SocketAddr,
}

impl Stopper {
    /// Tells the service to stop: it accepts nothing more, every wait ends, and the requests in
    /// flight finish. Answers at once; [`Server::run`] returns when they have.
    pub fn stop(&self) {
        self.signals.stop();
        // Wakes an `accept` that is waiting for a connection, so it sees the stop.
        let _ = TcpStream::connect_timeout(&self.address, Duration::from_secs(1));
    }
}

impl<B: Backend> Server<B> {
    /// Binds `config.address` and holds `backend` for the service.
    ///
    /// # Errors
    ///
    /// [`io::ErrorKind::InvalidInput`] for an address that is not loopback, refused before
    /// anything is bound: this stage serves on a loopback address only. Otherwise what binding
    /// refuses: the address is taken, or the port is privileged.
    pub fn bind(config: Config, backend: B) -> io::Result<Self> {
        if !config.address.ip().is_loopback() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "{} is not a loopback address: this stage serves on a loopback address only",
                    config.address
                ),
            ));
        }
        let listener = TcpListener::bind(config.address)?;
        let address = listener.local_addr()?;
        Ok(Self {
            listener,
            address,
            shared: Arc::new(Shared {
                backend,
                config,
                signals: Arc::default(),
            }),
        })
    }

    /// The bound address: with port 0 asked for, the port that was taken.
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }

    /// The handle that stops this service.
    #[must_use]
    pub fn stopper(&self) -> Stopper {
        Stopper {
            signals: Arc::clone(&self.shared.signals),
            address: self.address,
        }
    }

    /// Runs one sweep of the expiry task now, at the clock's reading, and answers the count of
    /// `Expire` commands it sent.
    pub fn sweep_now(&self) -> usize {
        sweep_now(&self.shared)
    }

    /// Serves until told to stop ([`Stopper::stop`]): accepts connections, each on its own thread,
    /// at most `max_connections` at once, and runs the expiry task every `sweep_interval`. Returns
    /// when every request in flight has been answered and the expiry task has ended.
    ///
    /// # Errors
    ///
    /// When the expiry task's thread cannot be started.
    pub fn run(self) -> io::Result<()> {
        let expiry = {
            let shared = Arc::clone(&self.shared);
            thread::Builder::new()
                .name("inbox-expiry".to_owned())
                .spawn(move || {
                    let interval = shared.config.sweep_interval.max(MIN_SWEEP_INTERVAL);
                    while shared.signals.sleep(interval) {
                        sweep_now(&shared);
                    }
                })?
        };
        let signals = &self.shared.signals;
        while let Some(slot) = signals.take_slot(self.shared.config.max_connections) {
            let stream = match self.listener.accept() {
                Ok((stream, _)) => stream,
                Err(_) => {
                    // A failed accept (no descriptor left) would fail again at once.
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
            };
            if signals.stopping() {
                break;
            }
            let shared = Arc::clone(&self.shared);
            // A thread that cannot be started drops the connection and its slot with it.
            let _ = thread::Builder::new()
                .name("inbox-connection".to_owned())
                .spawn(move || {
                    let _slot = slot;
                    connection(&shared, stream);
                });
        }
        signals.wait_idle();
        expiry
            .join()
            .map_err(|_| io::Error::other("the expiry task panicked"))
    }

    /// [`Server::run`] on a thread of its own, for an embedding or a test that goes on meanwhile.
    #[must_use]
    pub fn spawn(self) -> Running<B> {
        let address = self.address;
        let stopper = self.stopper();
        let shared = Arc::clone(&self.shared);
        let thread = thread::spawn(move || self.run());
        Running {
            address,
            stopper,
            shared,
            thread: Some(thread),
        }
    }
}

fn sweep_now<B: Backend>(shared: &Shared<B>) -> usize {
    let config = &shared.config;
    sweep(
        &shared.backend,
        &config.keys,
        config.clock.now(),
        config.answer_retention,
    )
}

/// A service serving on a thread of its own. Dropped, it is told to stop and not waited for.
#[derive(Debug)]
pub struct Running<B> {
    address: SocketAddr,
    stopper: Stopper,
    shared: Arc<Shared<B>>,
    thread: Option<JoinHandle<io::Result<()>>>,
}

impl<B: Backend> Running<B> {
    /// The bound address.
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }

    /// The handle that stops this service.
    #[must_use]
    pub fn stopper(&self) -> Stopper {
        self.stopper.clone()
    }

    /// Runs one sweep of the expiry task now, as [`Server::sweep_now`].
    pub fn sweep_now(&self) -> usize {
        sweep_now(&self.shared)
    }

    /// Stops the service and waits until [`Server::run`] has returned.
    ///
    /// # Errors
    ///
    /// What [`Server::run`] answered, or that its thread panicked.
    pub fn stop(mut self) -> io::Result<()> {
        self.stopper.stop();
        match self.thread.take() {
            Some(thread) => thread
                .join()
                .map_err(|_| io::Error::other("the service thread panicked"))?,
            None => Ok(()),
        }
    }
}

impl<B> Drop for Running<B> {
    fn drop(&mut self) {
        if self.thread.is_some() {
            self.stopper.stop();
        }
    }
}

/// One connection: read one request with the generated reader's limits, answer it, close.
fn connection<B: Backend>(shared: &Shared<B>, stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(http::READ_TIMEOUT));
    let _ = stream.set_write_timeout(Some(http::WRITE_TIMEOUT));
    let mut reader = BufReader::new(stream);
    let (answered, refused) = match http::read(&mut reader) {
        Ok(request) => (answer(shared, &request), false),
        Err(refusal) => (refusal, true),
    };
    let mut stream = reader.into_inner();
    // A request refused while it was read may still be arriving: read and drop the rest after the
    // answer, so closing does not reset the connection before the caller reads it.
    if write(&mut stream, &answered).is_ok() && refused {
        http::linger(&mut stream);
    }
}

/// Writes one answer as the generated `http::write` does, with the reason phrase of the two
/// statuses only this service answers (`401`, `503`).
fn write(stream: &mut TcpStream, answer: &http::Response) -> io::Result<()> {
    let reason = match answer.status {
        401 => "Unauthorized",
        503 => "Service Unavailable",
        status => http::reason(status),
    };
    let head = format!(
        "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        answer.status,
        answer.content_type,
        answer.body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(answer.body.as_bytes())?;
    stream.flush()
}
