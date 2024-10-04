use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU32, Ordering},
};

use atomic_wait::{wait, wake_one};

const UNLOCKED: u32 = 0;
const LOCKED: u32 = 1;

pub struct Mutex<T> {
    /// 0: unlocked
    /// 1: locked
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

    pub fn lock(&self) -> MutexGuard<T> {
        // set the state to locked
        while self.state.swap(LOCKED, Ordering::Acquire) == LOCKED {
            // if it was already locked, try to wait, until it is unlocked
            wait(&self.state, LOCKED);
        }
        MutexGuard { mutex: self }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        // set the state to unlocked
        self.mutex.state.store(UNLOCKED, Ordering::Release);
        // wake up one of the waiting threads
        wake_one(&self.mutex.state);
    }
}
