//! The command-level section: through `inbox_core::handle`, requests over one store run one at a
//! time, and the section is free again after a refusal and after a panic.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc;
use std::sync::{Arc, Barrier};
use std::time::Duration;

use inbox::actor::{Actor, Caller};
use inbox::behaviour::{ExternalCommand, MessageStorage, TryContext};
use inbox::messaging::{Address, MessageId, PostMessage, Principal};
use inbox::obligation::UnmetObligation;
use inbox::server::entry::Refused;
use inbox::server::json;
use inbox_core::{MemoryBackend, PostExternals};

use crate::support::{KeyIdVerifier, Post, Reply, World, open_worker, outcome, post, text};

const THREADS: usize = 8;
const ROUNDS: usize = 100;
const MESSAGE: &str = "01JGT0SC000000000000000001";

/// Runs `request` on `THREADS` threads released together, each over its own clone of the
/// backend, as a listener moves one to each connection, and answers each thread's outcome.
fn at_once(world: &World, request: impl Fn(&World, usize) -> String + Sync) -> Vec<String> {
    let barrier = Barrier::new(THREADS);
    std::thread::scope(|scope| {
        let threads: Vec<_> = (0..THREADS)
            .map(|n| {
                let barrier = &barrier;
                let request = &request;
                let own = World {
                    backend: world.backend.clone(),
                    verifier: world.verifier.clone(),
                };
                scope.spawn(move || {
                    barrier.wait();
                    request(&own, n)
                })
            })
            .collect();
        threads
            .into_iter()
            .map(|thread| thread.join().expect("the thread ran"))
            .collect()
    })
}

#[test]
fn posts_with_one_message_id_at_once_store_the_first_and_refuse_the_rest() {
    for round in 0..ROUNDS {
        let world = World::new(KeyIdVerifier::accepting());
        open_worker(&world);
        let answers = at_once(&world, |world, n| {
            let mut note = Post::note(MESSAGE);
            note.dedup_key = format!("key-{n}");
            note.body = format!("Post {n}.");
            format!("{n}:{}", outcome(&post(world, &note)))
        });
        let posted: Vec<&String> = answers.iter().filter(|a| a.ends_with(":posted")).collect();
        let duplicate = answers.iter().filter(|a| a.ends_with(":duplicate")).count();
        assert!(
            posted.len() == 1 && duplicate == THREADS - 1,
            "round {round}: {answers:?}"
        );
        let first = posted[0].split(':').next().expect("the thread number");
        let row = MessageStorage::get(&world.backend, &MessageId(MESSAGE.to_owned()))
            .expect("one row stored");
        assert_eq!(row.data.body.0, format!("Post {first}."), "round {round}");
        assert_eq!(
            row.data.dedup_key.0,
            format!("key-{first}"),
            "round {round}"
        );
    }
}

#[test]
fn answers_to_one_message_at_once_settle_it_once() {
    for round in 0..ROUNDS {
        let world = World::new(KeyIdVerifier::accepting());
        open_worker(&world);
        assert_eq!(outcome(&post(&world, &Post::request(MESSAGE, 3))), "posted");
        let answers = at_once(&world, |world, n| {
            let reply = Reply::to(MESSAGE, &format!("01JGT0SC00000000000000010{n}"));
            let answered =
                world.client("agent:box/worker", "inbox.messaging.Answer", &reply.json());
            format!("{n}:{}", outcome(&answered))
        });
        let answered: Vec<&String> = answers
            .iter()
            .filter(|a| a.ends_with(":answered"))
            .collect();
        let settled = answers
            .iter()
            .filter(|a| a.ends_with(":already-settled"))
            .count();
        assert!(
            answered.len() == 1 && settled == THREADS - 1,
            "round {round}: {answers:?}"
        );
        let first = answered[0].split(':').next().expect("the thread number");
        let rows = world.rows(
            "inbox.messaging.Settled",
            r#"{"sender": "agent:ws/planner"}"#,
        );
        assert_eq!(rows.len(), 1, "round {round}");
        let reply = rows[0].member("reply").expect("the reply is stored");
        assert_eq!(
            text(reply, "reply_id"),
            format!("01JGT0SC00000000000000010{first}"),
            "round {round}"
        );
    }
}

/// A context whose `PostMessage` panics while its command holds the section.
struct Panics;

impl TryContext for Panics {
    fn try_caller_inbox(&self) -> Result<Option<Address>, UnmetObligation> {
        Ok(None)
    }

    fn try_caller_principal(&self) -> Result<Option<Principal>, UnmetObligation> {
        Ok(None)
    }

    fn try_external(
        &mut self,
        _command: ExternalCommand<'_>,
        _outcome: &'static str,
    ) -> Result<bool, UnmetObligation> {
        Ok(false)
    }
}

impl PostExternals for Panics {
    fn unknown_option(&mut self, _input: &PostMessage) -> bool {
        panic!("a command panics inside the section")
    }

    fn signature_invalid(&mut self, _input: &PostMessage) -> bool {
        false
    }

    fn authority_invalid(&mut self, _input: &PostMessage) -> bool {
        false
    }
}

/// Opens an inbox through the entry on another thread, and fails the case if the section does not
/// let it in within 10 seconds.
fn next_request_runs(backend: &MemoryBackend, address: &str) {
    let backend = backend.clone();
    let address = address.to_owned();
    let (sent, answered) = mpsc::channel();
    std::thread::spawn(move || {
        let world = World {
            backend,
            verifier: KeyIdVerifier::accepting(),
        };
        let answer = world.client(
            &address,
            "inbox.messaging.OpenInbox",
            &format!("{{\"address\":\"{address}\",\"adapter\":\"terminal\"}}"),
        );
        let _ = sent.send(outcome(&answer).to_owned());
    });
    let outcome = answered
        .recv_timeout(Duration::from_secs(10))
        .expect("the next request ran: the section was released");
    assert_eq!(outcome, "opened");
}

#[test]
fn the_entry_releases_the_section_after_a_refusal_and_after_a_panic() {
    let world = World::new(KeyIdVerifier::accepting());
    open_worker(&world);

    let mut nowhere = Post::note(MESSAGE);
    nowhere.to = "agent:box/nobody".to_owned();
    assert_eq!(outcome(&post(&world, &nowhere)), "unknown-inbox");
    next_request_runs(&world.backend, "agent:box/one");

    let context = inbox_core::ServiceContext::new(
        inbox_core::CallerAttributes::none(),
        KeyIdVerifier::accepting(),
        world.backend.clone(),
    );
    let refused = inbox_core::handle(
        &world.backend,
        context,
        Some(&Caller {
            actor: Actor::Sweeper,
        }),
        "inbox.messaging.PostMessage",
        json::parse(&Post::note(MESSAGE).json()).expect("JSON"),
    );
    assert!(
        matches!(refused, Err(Refused::NotGranted(_))),
        "{refused:?}"
    );
    next_request_runs(&world.backend, "agent:box/two");

    let backend = Arc::new(world.backend.clone());
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        inbox_core::handle(
            backend.as_ref(),
            Panics,
            Some(&Caller {
                actor: Actor::Client,
            }),
            "inbox.messaging.PostMessage",
            json::parse(&Post::note(MESSAGE).json()).expect("JSON"),
        )
    }));
    assert!(panicked.is_err(), "the command panicked");
    next_request_runs(&world.backend, "agent:box/three");
    assert!(
        MessageStorage::get(&world.backend, &MessageId(MESSAGE.to_owned())).is_none(),
        "the panicking post stored nothing"
    );
}
