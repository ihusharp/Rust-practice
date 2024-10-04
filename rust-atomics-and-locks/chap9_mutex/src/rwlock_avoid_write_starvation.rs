// For our mutex, we needed to track only whether it was locked or not.
// For our reader-writer lock, however, we also need to know how many (reader) locks are currently held,
// to make sure write-locking only happens after all readers have released their locks.

use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU32, Ordering},
};

use atomic_wait::{wait, wake_all, wake_one};

// keep track of whether there are any waiting writers.
pub struct RwLock<T> {
    // multiply the reader count by 2
    // add 1 for situations where there is a writer waiting.
    // u32::MAX if a writer has locked the lock.
    //
    // This means that readers may acquire the lock when
    // the state is *even*, but need to block when *odd*.
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
            if s % 2 == 0 {
                // all are readers
                assert!(s < u32::MAX - 2, "too many readers");
                match self.state.compare_exchange_weak(
                    s,
                    s + 2,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return ReadGuard { rwlock: self },
                    Err(e) => s = e,
                }
            }

            // which means a writer has locked the lock or a writer is waiting
            if s % 2 == 1 {
                wait(&self.state, u32::MAX);
                s = self.state.load(Ordering::Relaxed);
            }
        }
    }

    pub fn write(&self) -> WriteGuard<T> {
        let mut s = self.state.load(Ordering::Relaxed);
        loop {
            // If the state is 0 or 1, which means the RwLock is unlocked
            // because 1 is only indicate a writer is waiting.
            if s <= 1 {
                match self.state.compare_exchange_weak(
                    s,
                    u32::MAX,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return WriteGuard { rwlock: self },
                    Err(e) => {
                        s = e;
                        continue;
                    }
                }
            }
            // Block new readers, by making sure the state is odd.
            if s % 2 == 0 {
                match self
                    .state
                    .compare_exchange(s, s + 1, Ordering::Relaxed, Ordering::Relaxed)
                {
                    Ok(_) => {}
                    Err(e) => {
                        s = e;
                        continue;
                    }
                }
            }
            // for now we know that the state is odd, 
            // which means a writer is waiting, and the RwLock is still locked
            let w = self.writer_wake_counter.load(Ordering::Acquire);
            s = self.state.load(Ordering::Relaxed);
            if s >= 2 {
                // we wait for the writer_wake_counter variable,
                // while making sure that the lock hasn’t been unlocked in the meantime.
                // drop read will wake up the writer(when from 3 to 1)
                wait(&self.state, w);
                s = self.state.load(Ordering::Relaxed);
            }
        }
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
        // Decrement the state by 2 to remove one read-lock.
        if self.rwlock.state.fetch_sub(2, Ordering::Release) == 3 {
            // If we decremented from 3 to 1, that means
            // the RwLock is now unlocked _and_ there is
            // a waiting writer, which we wake up.
            self.rwlock
                .writer_wake_counter
                .fetch_add(1, Ordering::Release);
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
