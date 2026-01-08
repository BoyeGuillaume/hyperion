use std::{
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
};

use crossbeam::utils::Backoff;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};

const STATE_DIRTY: usize = 0; // Value is dirty and not being locked
const STATE_DIRTY_LOCK: usize = 1; // Value is being dirtied (should not compute)
const STATE_COMPUTING: usize = 2; // Value is behing computed (should not dirtify nor compute nor read)
const STATE_CLEAN: usize = 3; // Value is clean and can be used or dirtified
// Everything above STATE_CLEAN is counting number of "in use" witch should prevent dirtifying

/// Guard for a lazy value
///
/// This is akin to a reference `&'a T` however it also enforces that we cannot modify the associated
/// element while this guard is active.
pub struct LazyGuard<'a, K: ?Sized> {
    guard: MappedRwLockReadGuard<'a, K>,
    state: &'a AtomicUsize,
}

impl<'a, K: ?Sized> AsRef<K> for LazyGuard<'a, K> {
    fn as_ref(&self) -> &'_ K {
        &*self.guard
    }
}

impl<'a, K: ?Sized> Deref for LazyGuard<'a, K> {
    type Target = K;

    fn deref(&self) -> &'_ Self::Target {
        &*self.guard
    }
}

impl<'a, K: ?Sized> Drop for LazyGuard<'a, K> {
    fn drop(&mut self) {
        let backoff = Backoff::new();

        loop {
            // Load the previous state
            let prev_state = self.state.load(Ordering::Acquire);
            debug_assert!(
                prev_state > STATE_CLEAN,
                "LazyGuard state should be CLEAN_IN_USE when dropping"
            );

            // Decrement the in-use counter
            let new_state = prev_state - 1;
            if self
                .state
                .compare_exchange(prev_state, new_state, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break;
            } else {
                backoff.spin();
            }
        }
    }
}

pub struct LazyContainer<T> {
    elem: RwLock<Option<T>>,
    state: AtomicUsize,
}

impl<T> LazyContainer<T> {
    pub const fn new() -> Self {
        Self {
            elem: RwLock::new(None),
            state: AtomicUsize::new(STATE_DIRTY),
        }
    }

    pub fn dirtify<'a, E>(&'a self, elem: &'a mut E) -> LazyDirtifierGuard<'a, E> {
        // Can only dirtify if the state is either DIRTY or CLEAN (not CLEAN_IN_USE)
        let backoff = Backoff::new();
        loop {
            let prev_state = self.state.load(Ordering::Acquire);

            if matches!(prev_state, STATE_DIRTY | STATE_CLEAN) {
                if self
                    .state
                    .compare_exchange(
                        prev_state,
                        STATE_DIRTY_LOCK,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_ok()
                {
                    break;
                }
            } else {
                // Wait until the state is DIRTY or CLEAN
                backoff.snooze();
            }
        }

        LazyDirtifierGuard {
            state: &self.state,
            elem,
        }
    }

    pub fn get<'a, K: ?Sized>(
        &'a self,
        compute: impl FnOnce(Option<T>) -> T,
        map: impl FnOnce(&T) -> &K,
    ) -> LazyGuard<'a, K> {
        // Here is the logic:
        // - If the state is CLEAN, return the value
        // - If the state is DIRTY, compute the value and set the state to CLEAN
        // - If any other state, wait
        let backoff = Backoff::new();

        loop {
            let prev_state = self.state.load(Ordering::Acquire);

            match prev_state {
                STATE_CLEAN..=usize::MAX => {
                    // Attempt to increment the in-use counter
                    let added = prev_state + 1;
                    if self
                        .state
                        .compare_exchange(prev_state, added, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                    {
                        // Return the value
                        let guard = self.elem.read();
                        let mapped = RwLockReadGuard::map(guard, |x| {
                            let raw = x
                                .as_ref()
                                .expect("Value should be present when state is CLEAN or higher");
                            map(raw)
                        });
                        return LazyGuard {
                            guard: mapped,
                            state: &self.state,
                        };
                    }
                }
                STATE_DIRTY => {
                    if self
                        .state
                        .compare_exchange(
                            STATE_DIRTY,
                            STATE_COMPUTING,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        )
                        .is_ok()
                    {
                        // Retrieve the previous value (useful to avoid reallocations and for incremental computations)
                        let mut mut_guard = self.elem.write();
                        let previous_value = mut_guard.take();

                        // We can compute the value
                        let new_value = compute(previous_value);

                        // Store the new value
                        *mut_guard = Some(new_value);
                        self.state
                            .compare_exchange(
                                STATE_COMPUTING,
                                STATE_CLEAN + 1, // Mark as in-use
                                Ordering::AcqRel,
                                Ordering::Acquire,
                            )
                            .expect("State should be COMPUTING when finishing computation");

                        // Return the value (and rerun the loop to get it properly)
                        drop(mut_guard); // Release the write lock

                        let guard = self.elem.read();
                        let mapped = RwLockReadGuard::map(guard, |x| {
                            let raw = x
                                .as_ref()
                                .expect("Value should be present when state is CLEAN or higher");
                            map(raw)
                        });
                        return LazyGuard {
                            guard: mapped,
                            state: &self.state,
                        };
                    }
                }
                _ => {
                    // Wait until the state is either DIRTY or CLEAN or higher (CLEAN_IN_USE)
                    backoff.snooze();
                }
            }
        }
    }

    pub fn get_simple<'a>(&'a self, compute: impl FnOnce(Option<T>) -> T) -> LazyGuard<'a, T> {
        self.get(compute, |x| x)
    }
}

/// Dirty guard for a lazy value
pub struct LazyDirtifierGuard<'a, E> {
    state: &'a AtomicUsize,
    elem: &'a mut E,
}

impl<'a, E> AsRef<E> for LazyDirtifierGuard<'a, E> {
    fn as_ref(&self) -> &'_ E {
        self.elem
    }
}

impl<'a, E> Deref for LazyDirtifierGuard<'a, E> {
    type Target = E;

    fn deref(&self) -> &'_ Self::Target {
        self.elem
    }
}

impl<'a, E> DerefMut for LazyDirtifierGuard<'a, E> {
    fn deref_mut(&mut self) -> &'_ mut Self::Target {
        self.elem
    }
}

impl<'a, E> Drop for LazyDirtifierGuard<'a, E> {
    fn drop(&mut self) {
        self.state
            .compare_exchange(
                STATE_DIRTY_LOCK,
                STATE_DIRTY,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .expect("LazyGuard state should be DIRTY_LOCK when dropping Dirty");
    }
}
