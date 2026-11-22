//! What the stand-in services of the cases share: a bounded wait for a connection, so a case
//! whose client never connects ends its stand-in with a message that names what did not arrive,
//! and not at the runner's kill after 60 s.

use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

/// How long a stand-in waits for each connection.
pub const STAND_IN_WAIT: Duration = Duration::from_secs(10);

/// The bound of the acceptance: a stand-in whose client never connects ends within 15 s. A longer
/// `STAND_IN_WAIT` does not compile.
const _: () = assert!(STAND_IN_WAIT.as_secs() <= 15);

/// The next connection on `listener`, or the message that none came for `what` within `wait`.
pub fn accept_within(
    listener: &TcpListener,
    wait: Duration,
    what: &str,
) -> Result<TcpStream, String> {
    listener
        .set_nonblocking(true)
        .expect("the stand-in's listener does not block");
    let deadline = Instant::now() + wait;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream
                    .set_nonblocking(false)
                    .expect("the stand-in's connection blocks");
                return Ok(stream);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(format!(
                        "the stand-in received no connection for {what} within {} s",
                        wait.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("the stand-in's listener failed: {error}"),
        }
    }
}

/// A stand-in whose client never connects ends and says what it waited for, within its wait.
#[test]
fn a_wait_for_a_connection_that_never_comes_ends_and_names_it() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
    let started = Instant::now();
    let message = accept_within(&listener, Duration::from_secs(1), "the request of the case")
        .expect_err("no client connects");
    assert_eq!(
        message,
        "the stand-in received no connection for the request of the case within 1 s"
    );
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "{:?}",
        started.elapsed()
    );
}
