// For our mutex, we needed to track only whether it was locked or not.
// For our reader-writer lock, however, we also need to know how many (reader) locks are currently held,
// to make sure write-locking only happens after all readers have released their locks.

use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU32, Ordering},
};

use atomic_wait::{wait, wake_all, wake_one};

pub struct RwLock<T> {
    // The number of readers, or u32::MAX if a writer has locked the lock.
    state: AtomicU32,
    value: UnsafeCell<T>,
    // Incremented to wake up writers
    writer_wake_counter: AtomicU32,
}

unsafe impl<T> Sync for RwLock<T> where T: Send + Sync {}

impl<T> RwLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            value: UnsafeCell::new(value),
            writer_wake_counter: AtomicU32::new(0),
        }
    }

    pub fn read(&self) -> ReadGuard<T> {
        let mut s = self.state.load(Ordering::Relaxed);
        loop {
            if s < u32::MAX {
                assert!(s < u32::MAX - 1, "too many readers");
                match self.state.compare_exchange_weak(
                    s,
                    s + 1,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return ReadGuard { rwlock: self },
                    Err(e) => s = e,
                }
            }

            // which means a writer has locked the lock
            if s == u32::MAX {
                wait(&self.state, u32::MAX);
                s = self.state.load(Ordering::Relaxed);
            }
        }
    }

    pub fn write(&self) -> WriteGuard<T> {
        while self
            .state
            .compare_exchange(0, u32::MAX, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            let w = self.writer_wake_counter.load(Ordering::Acquire);
            if self.state.load(Ordering::Relaxed) != 0 {
                // Wait if the RwLock is still locked, but only if
                // there have been no wake signals since we checked.
                wait(&self.state, w)
            }
        }
        WriteGuard { rwlock: self }
    }
}

pub struct ReadGuard<'a, T> {
    rwlock: &'a RwLock<T>,
}

// read guard only needs to dereference the value, not DerefMut
// because it doesn't have exclusive access to the value
impl<T> Deref for ReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.rwlock.value.get() }
    }
}

impl<T> Drop for ReadGuard<'_, T> {
    fn drop(&mut self) {
        if self.rwlock.state.fetch_sub(1, Ordering::Release) == 1 {
            self.rwlock
                .writer_wake_counter
                .fetch_add(1, Ordering::Release);
            // wake a writer if there is one.
            // Waking up just one thread is enough,
            // because there cannot be any waiting readers at this point.
            wake_one(&self.rwlock.state);
        }
    }
}

pub struct WriteGuard<'a, T> {
    rwlock: &'a RwLock<T>,
}

impl<T> Deref for WriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.rwlock.value.get() }
    }
}

impl<T> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.rwlock.value.get() }
    }
}

// Since we still don’t know whether there are writers or readers waiting,
// we have to wake both one waiting writer (through wake_one) and all waiting readers (using wake_all):
impl<T> Drop for WriteGuard<'_, T> {
    fn drop(&mut self) {
        self.rwlock.state.store(0, Ordering::Release);
        self.rwlock
            .writer_wake_counter
            .fetch_add(1, Ordering::Release);
        wake_one(&self.rwlock.writer_wake_counter);
        // wake up all waiting threads
        wake_all(&self.rwlock.state);
    }
}
