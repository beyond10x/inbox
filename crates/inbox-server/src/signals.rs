//! What the threads of one service tell each other: a command was answered, a connection slot is
//! free, a request has not arrived in time, the service stops. One lock and one condition variable;
//! each waiter checks its own state.

use std::collections::BTreeMap;
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
struct State {
    /// Counts the commands answered; a wait reads again when it moved.
    generation: u64,
    /// The service stops: nothing more is accepted, every wait ends.
    stopping: bool,
    /// The connections being served.
    active: usize,
    /// The id the next watched connection takes.
    next_arrival: u64,
    /// The connections whose request has not arrived in full yet.
    arrivals: BTreeMap<u64, Arrival>,
}

/// Why a connection's request was cut off before it arrived in full.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cut {
    /// The request did not arrive in full within `request_timeout`.
    TimedOut,
    /// The service stops.
    Stopping,
}

const NOT_CUT: u8 = 0;
const TIMED_OUT: u8 = 1;
const STOPPING: u8 = 2;

/// A watched connection: a handle on its socket, and when its request must have arrived.
#[derive(Debug)]
struct Arrival {
    stream: TcpStream,
    deadline: Option<Instant>,
    cut: Arc<AtomicU8>,
}

impl Arrival {
    /// Records why, then shuts the socket for reading: a read blocked on it returns at once with
    /// the end of the stream, and the generated reader refuses the request it has not finished.
    fn cut(&self, why: u8) {
        let _ = self
            .cut
            .compare_exchange(NOT_CUT, why, Ordering::SeqCst, Ordering::SeqCst);
        let _ = self.stream.shutdown(Shutdown::Read);
    }
}

#[derive(Debug, Default)]
pub(crate) struct Signals {
    state: Mutex<State>,
    moved: Condvar,
}

impl Signals {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Changes the state under the lock and wakes every waiter, so none misses it.
    fn update<R>(&self, change: impl FnOnce(&mut State) -> R) -> R {
        let changed = change(&mut self.state());
        self.moved.notify_all();
        changed
    }

    /// Waits on the condition until `done` holds or `deadline` passes (no deadline: until `done`
    /// holds); answers `done` at the end.
    fn wait_until(&self, deadline: Option<Instant>, done: impl Fn(&State) -> bool) -> bool {
        let mut state = self.state();
        loop {
            if done(&state) {
                return true;
            }
            state = match deadline {
                None => self
                    .moved
                    .wait(state)
                    .unwrap_or_else(PoisonError::into_inner),
                Some(deadline) => {
                    let left = deadline.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        return false;
                    }
                    self.moved
                        .wait_timeout(state, left)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0
                }
            };
        }
    }

    /// The count of commands answered so far.
    pub(crate) fn generation(&self) -> u64 {
        self.state().generation
    }

    /// A command was answered: every wait reads its view again.
    pub(crate) fn command_answered(&self) {
        self.update(|state| state.generation = state.generation.wrapping_add(1));
    }

    /// Waits until a command is answered after `seen`, the service stops, or `deadline` passes
    /// (no deadline: one too far away to name); `true` only for a command answered while the
    /// service still runs.
    pub(crate) fn wait_for_command(&self, seen: u64, deadline: Option<Instant>) -> bool {
        self.wait_until(deadline, |state| state.stopping || state.generation != seen)
            && !self.stopping()
    }

    /// The service stops: every wait ends, and every connection whose request has not arrived in
    /// full is cut off.
    pub(crate) fn stop(&self) {
        self.update(|state| {
            state.stopping = true;
            for arrival in std::mem::take(&mut state.arrivals).into_values() {
                arrival.cut(STOPPING);
            }
        });
    }

    pub(crate) fn stopping(&self) -> bool {
        self.state().stopping
    }

    /// Sleeps for `interval` or until the service stops; `true` while it still runs. An interval
    /// too long to add to the clock sleeps until the service stops.
    pub(crate) fn sleep(&self, interval: Duration) -> bool {
        let deadline = Instant::now().checked_add(interval);
        !self.wait_until(deadline, |state| state.stopping)
    }

    /// Takes a connection slot, waiting while `most` are taken; `None` when the service stops.
    pub(crate) fn take_slot(self: &Arc<Self>, most: usize) -> Option<Slot> {
        let mut state = self.state();
        loop {
            if state.stopping {
                return None;
            }
            if state.active < most.max(1) {
                state.active += 1;
                return Some(Slot(Arc::clone(self)));
            }
            state = self
                .moved
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Waits until every connection slot is free again.
    pub(crate) fn wait_idle(&self) {
        self.wait_until(None, |state| state.active == 0);
    }

    /// Watches `stream` until the returned [`Watch`] is dropped: its read side is shut when
    /// `bound` has passed (a bound too long to add to the clock never passes) or when the service
    /// stops. A socket that cannot be cloned is not watched; the reader's gap bound still holds.
    pub(crate) fn watch(self: &Arc<Self>, stream: &TcpStream, bound: Duration) -> Watch {
        let cut = Arc::new(AtomicU8::new(NOT_CUT));
        let deadline = Instant::now().checked_add(bound);
        let handle = stream.try_clone().ok();
        let id = self.update(|state| {
            let id = state.next_arrival;
            state.next_arrival = state.next_arrival.wrapping_add(1);
            if let Some(stream) = handle {
                let arrival = Arrival {
                    stream,
                    deadline,
                    cut: Arc::clone(&cut),
                };
                if state.stopping {
                    arrival.cut(STOPPING);
                } else {
                    state.arrivals.insert(id, arrival);
                }
            }
            id
        });
        Watch {
            signals: Arc::clone(self),
            id,
            cut,
        }
    }

    /// The body of the thread that cuts off every watched connection at its deadline. Returns when
    /// the service stops, which cuts off the rest itself.
    pub(crate) fn reap(&self) {
        let mut state = self.state();
        loop {
            if state.stopping {
                return;
            }
            let now = Instant::now();
            let due: Vec<u64> = state
                .arrivals
                .iter()
                .filter(|(_, arrival)| arrival.deadline.is_some_and(|deadline| deadline <= now))
                .map(|(id, _)| *id)
                .collect();
            for id in due {
                if let Some(arrival) = state.arrivals.remove(&id) {
                    arrival.cut(TIMED_OUT);
                }
            }
            let next = state
                .arrivals
                .values()
                .filter_map(|arrival| arrival.deadline)
                .min();
            state = match next {
                None => self
                    .moved
                    .wait(state)
                    .unwrap_or_else(PoisonError::into_inner),
                Some(next) => {
                    self.moved
                        .wait_timeout(state, next.saturating_duration_since(now))
                        .unwrap_or_else(PoisonError::into_inner)
                        .0
                }
            };
        }
    }
}

/// One connection slot, given back when dropped, a panic of its connection included.
#[derive(Debug)]
pub(crate) struct Slot(Arc<Signals>);

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.update(|state| state.active -= 1);
    }
}

/// A connection under watch until this is dropped (`Signals::watch`).
#[derive(Debug)]
pub(crate) struct Watch {
    signals: Arc<Signals>,
    id: u64,
    cut: Arc<AtomicU8>,
}

impl Watch {
    /// Why the connection was cut off, if it was.
    pub(crate) fn cut(&self) -> Option<Cut> {
        match self.cut.load(Ordering::SeqCst) {
            TIMED_OUT => Some(Cut::TimedOut),
            STOPPING => Some(Cut::Stopping),
            _ => None,
        }
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.signals.state().arrivals.remove(&self.id);
    }
}
