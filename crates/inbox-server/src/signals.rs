//! What the threads of one service tell each other: a command was answered, a connection slot is
//! free, the service stops. One lock and one condition variable; each waiter checks its own state.

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
    fn update(&self, change: impl FnOnce(&mut State)) {
        change(&mut self.state());
        self.moved.notify_all();
    }

    /// Waits on the condition until `done` holds or `deadline` passes; answers `done` at the end.
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

    /// Waits until a command is answered after `seen`, the service stops, or `deadline` passes;
    /// `true` only for a command answered while the service still runs.
    pub(crate) fn wait_for_command(&self, seen: u64, deadline: Instant) -> bool {
        self.wait_until(Some(deadline), |state| {
            state.stopping || state.generation != seen
        }) && !self.stopping()
    }

    pub(crate) fn stop(&self) {
        self.update(|state| state.stopping = true);
    }

    pub(crate) fn stopping(&self) -> bool {
        self.state().stopping
    }

    /// Sleeps for `interval` or until the service stops; `true` while it still runs.
    pub(crate) fn sleep(&self, interval: Duration) -> bool {
        let deadline = Instant::now() + interval;
        !self.wait_until(Some(deadline), |state| state.stopping)
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
}

/// One connection slot, given back when dropped, a panic of its connection included.
#[derive(Debug)]
pub(crate) struct Slot(Arc<Signals>);

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.update(|state| state.active -= 1);
    }
}
