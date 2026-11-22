//! Adversary cases of story:repost-does-not-redeliver-a-handled-note (stage 2, wave 8, pass 1):
//! the `Inbox-Epoch` header the listener writes. No key material is written into this file.

use crate::support;

use support::World;

/// The epoch an unauthenticated answer of the service at `address` names.
fn epoch(address: std::net::SocketAddr) -> String {
    let (head, reply) =
        support::send_raw_answer(address, "GET", "/messaging/views/inboxes", &[], b"");
    assert_eq!(reply.status, 401, "{}", reply.body);
    let values = support::header(&head, "Inbox-Epoch");
    assert_eq!(values.len(), 1, "{head}");
    values[0].clone()
}

/// The epoch carries no time: 16 random bytes. 8 starts in quick succession answer 8 values of
/// which no 2 share their first 8 hex digits (for random values, a chance of about 7 in 10^9). A
/// value made from a clock reading shares its leading digits with the next start's.
#[test]
fn eight_starts_answer_epochs_that_share_no_leading_digits() {
    let world = World::new();
    let servers: Vec<_> = (0..8).map(|_| world.start(|_| {})).collect();
    let epochs: Vec<String> = servers
        .iter()
        .map(|server| epoch(server.local_addr()))
        .collect();
    for (at, one) in epochs.iter().enumerate() {
        for other in &epochs[at + 1..] {
            assert_ne!(
                one[..8],
                other[..8],
                "two starts share leading digits: {epochs:?}"
            );
        }
    }
    for server in servers {
        server.stop().expect("the service stops");
    }
}
