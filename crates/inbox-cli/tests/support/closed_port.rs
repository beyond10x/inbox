//! A loopback port where nothing answers, held by the case that needs it.
//!
//! The cases of the binary run at once, and every service of a case listens on a port the kernel
//! picks. A port bound and given back can go to the service of another case, which then answers
//! where the case expected a refused connect. [`ClosedPort`] keeps the port instead: its socket is
//! bound and never listened on, so a connect is refused and the kernel gives the port to no other
//! socket until the value is dropped.

use std::net::{Ipv4Addr, SocketAddrV4};

use rustix::fd::OwnedFd;
use rustix::net::{AddressFamily, SocketFlags, SocketType};

/// A TCP socket bound to 127.0.0.1 on a port the kernel picks, never listened on. While it lives
/// a connect to [`ClosedPort::url`] is refused (`ECONNREFUSED`) and no other socket is given the
/// port. Dropping it gives the port back.
pub struct ClosedPort {
    socket: OwnedFd,
    port: u16,
}

impl ClosedPort {
    pub fn new() -> Self {
        // Close on exec: a child the case starts must not keep the port after the case is done.
        let socket = rustix::net::socket_with(
            AddressFamily::INET,
            SocketType::STREAM,
            SocketFlags::CLOEXEC,
            None,
        )
        .expect("a TCP socket");
        rustix::net::bind(&socket, &SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .expect("the socket binds to a loopback port");
        let bound = rustix::net::getsockname(&socket).expect("the bound address");
        let port = SocketAddrV4::try_from(bound)
            .expect("an IPv4 address")
            .port();
        Self { socket, port }
    }

    /// The port held.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// `http://127.0.0.1:<port>`: a URL where nothing answers.
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl Default for ClosedPort {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ClosedPort {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ClosedPort")
            .field("socket", &self.socket)
            .field("port", &self.port)
            .finish()
    }
}

/// While a [`ClosedPort`] lives a second bind of its port fails and a connect to it is refused.
/// The case does not bind the port again after the drop: a listener of another case that asks
/// for any port could be given it in between.
#[test]
fn a_closed_port_is_held_while_its_value_lives() {
    let closed = ClosedPort::new();
    let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, closed.port());
    let second = std::net::TcpListener::bind(address);
    assert_eq!(
        second.as_ref().map_err(std::io::Error::kind).err(),
        Some(std::io::ErrorKind::AddrInUse),
        "a second bind of the held port {address}: {second:?}"
    );
    let connect = std::net::TcpStream::connect(address);
    assert_eq!(
        connect.as_ref().map_err(std::io::Error::kind).err(),
        Some(std::io::ErrorKind::ConnectionRefused),
        "a connect to the held port {address}: {connect:?}"
    );
    drop(closed);
}
