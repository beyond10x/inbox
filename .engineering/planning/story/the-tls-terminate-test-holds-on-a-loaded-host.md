---
format: aep.planning-md/3
id: story:the-tls-terminate-test-holds-on-a-loaded-host
kind: story
status: draft
title: The TLS terminate test holds on a loaded host
summary: A unit case of the TLS listener decides a full buffer from a 500 ms stall and failed once in 4 runs under load
revision: 1
---
## Outcome

The unit case `tls::tests::terminate_returns_only_when_the_thread_that_reads_the_caller_has_ended`
of `crates/inbox-server/src/tls.rs` passes on a loaded host as on an idle one.

## Why

The case failed once in 4 runs of `cargo nextest run -p inbox-cli -p inbox-server --locked`
on 2026-11-23, on a host with a load average of about 70 to 90 on 20 CPUs, with "terminate
returned while the thread that reads the caller was still writing". It was found while
`story:inbox-wake-returns-on-a-post` was built and is older than that story: the file was not
changed by it. The case decides that the way in is full when writes stall for 500 ms. A loaded
host stalls a write for that long without a full way in, so the case then goes on from a state
it did not reach.

## Acceptance

- The case decides "the way in is full" from a fact and not from a pause of a fixed length
  (for example a write that would block on a socket set to non-blocking, or a counted number
  of bytes over the buffer sizes it set).
- It holds no wall-clock bound that a slow host breaks; a bounded wait only ends a hang.
- A mutant that makes `terminate` return before the reading thread has ended is still red
  against it.
- 20 runs of the case in a row are green while another build occupies the pool.

## Scope

`crates/inbox-server/src/tls.rs`, the test module only. No change of the listener.

## Decisions

None open.
