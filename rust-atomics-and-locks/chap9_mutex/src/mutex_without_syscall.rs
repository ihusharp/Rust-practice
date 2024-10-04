use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU32, Ordering},
};

use atomic_wait::{wait, wake_one};

const UNLOCKED: u32 = 0;
const LOCKED_SELF: u32 = 1; // locked, no other threads waiting
const LOCKED_OTHERS: u32 = 2; // locked, other threads waiting

pub struct Mutex<T> {
    /// 0: unlocked
    /// 1: locked, no other threads waiting
    /// 2: locked, other threads waiting
    state: AtomicU32,
    value: UnsafeCell<T>,
}

unsafe impl<T> Sync for Mutex<T> where T: Send {}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

unsafe impl<'a, T> Send for MutexGuard<'a, T> where T: Send {}
unsafe impl<'a, T> Sync for MutexGuard<'a, T> where T: Send {}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T> Mutex<T> {
    pub const fn new(value: T) -> Self {
        Self {
            state: AtomicU32::new(UNLOCKED),
            value: UnsafeCell::new(value),
        }
    }

    /// for an unlocked mutex, our lock function still needs to set the state to 1 to lock it.
    /// However, if it was already locked, our lock function now needs to set the state to 2 before going to sleep,
    /// so that the unlock function can tell there’s a waiting thread.
    pub fn lock(&self) -> MutexGuard<T> {
        if self
            .state
            .compare_exchange(UNLOCKED, LOCKED_SELF, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // if return value is not equal to UNLOCKED, means it still need to be locked.
            // why set to LOCKED_OTHERS?
            // because we need to tell the unlock function that there are other threads waiting.(maybe just one)
            // can read https://marabos.nl/atomics/building-locks.html#happens-before-diagram-mutex for more details
            while self.state.swap(LOCKED_OTHERS, Ordering::Acquire) != UNLOCKED {
                wait(&self.state, LOCKED_OTHERS);
            }
            // If the swap operation returns 0,
            // that means we’ve successfully locked the mutex by changing its state from 0 to 2.
        }

        MutexGuard { mutex: self }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        // set the state to unlocked
        // if the state was 1 which means no other threads waiting, we can set it to 0 directly.
        // both the wait() and wake_one() calls are entirely avoided.
        if self.mutex.state.swap(UNLOCKED, Ordering::Release) == LOCKED_OTHERS {
            // if the state was 2, wake up one of the waiting threads
            wake_one(&self.mutex.state);
        }
    }
}
