//! The expiry task's one sweep: `Due` at a clock reading, then `Expire` for each due message.

use std::sync::Arc;
use std::time::Duration;

use inbox::actor::{Actor, Caller};
use inbox::server::json::Value;
use inbox_core::{CallerAttributes, ServiceContext};
use inbox_verify::{Instant, KeyRing};

use crate::auth::RingVerifier;
use crate::clock::format_instant;
use crate::service::Backend;

/// The expiry task is the specification's `inbox.messaging.Sweeper`.
const SWEEPER: Caller = Caller {
    actor: Actor::Sweeper,
};

fn object(members: &[(&str, &str)]) -> Value {
    Value::Object(
        members
            .iter()
            .map(|(name, text)| ((*name).to_owned(), Value::Text((*text).to_owned())))
            .collect(),
    )
}

/// One sweep at the reading `now`: reads `Due` at `now` as the sweeper and sends `Expire` for each
/// due message, with `now` and `keep_until` = `now` + `retention` (to the second), each through
/// [`inbox_core::handle`] inside the backend's section. Answers the count of `Expire` commands
/// sent; what each did (removed, decided on silence, not due any more) is the core's to decide.
pub fn sweep<B: Backend>(
    backend: &B,
    keys: &Arc<KeyRing>,
    now: Instant,
    retention: Duration,
) -> usize {
    let reading = format_instant(now);
    let keep_until =
        format_instant(now.plus_seconds(i64::try_from(retention.as_secs()).unwrap_or(i64::MAX)));
    let context = || {
        ServiceContext::new(
            CallerAttributes::none(),
            RingVerifier::new(Arc::clone(keys), now),
            backend.clone(),
        )
    };
    let due = inbox_core::handle(
        backend,
        context(),
        Some(&SWEEPER),
        "inbox.messaging.Due",
        object(&[("now", &reading)]),
    );
    let ids: Vec<String> = match due.as_ref().ok().and_then(|due| due.member("rows")) {
        Some(Value::Array(rows)) => rows
            .iter()
            .filter_map(|row| match row.member("message_id") {
                Some(Value::Text(id)) => Some(id.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    for id in &ids {
        // A refusal of one Expire (the message went in between) leaves the others to run.
        let _ = inbox_core::handle(
            backend,
            context(),
            Some(&SWEEPER),
            "inbox.messaging.Expire",
            object(&[
                ("message_id", id),
                ("now", &reading),
                ("keep_until", &keep_until),
            ]),
        );
    }
    ids.len()
}
