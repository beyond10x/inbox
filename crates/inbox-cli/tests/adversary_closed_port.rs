//! Adversary case of story:a-case-that-needs-a-closed-port-keeps-the-port (pass 1): a child a case
//! starts while a [`ClosedPort`] lives does not inherit its socket, so a child that outlives the
//! case does not keep the port. Every socket is on loopback; no request leaves this host.

use std::fs;
use std::process::{Command, Stdio};

use crate::support::ClosedPort;

/// A child started while a [`ClosedPort`] lives holds no socket: the held socket is close on
/// exec. Kills the mutant without `SocketFlags::CLOEXEC`, which the case of `support` lets live.
#[test]
fn a_child_started_while_a_port_is_held_does_not_inherit_its_socket() {
    let closed = ClosedPort::new();
    let mut child = Command::new("sleep")
        .arg("5")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("sleep starts");
    let descriptors = fs::read_dir(format!("/proc/{}/fd", child.id()))
        .expect("the child's descriptors")
        .filter_map(|entry| fs::read_link(entry.ok()?.path()).ok())
        .map(|target| target.to_string_lossy().into_owned())
        .collect::<Vec<String>>();
    let _ = child.kill();
    let _ = child.wait();
    drop(closed);
    assert!(
        descriptors
            .iter()
            .all(|target| !target.starts_with("socket:")),
        "the child holds a socket: {descriptors:?}"
    );
}
