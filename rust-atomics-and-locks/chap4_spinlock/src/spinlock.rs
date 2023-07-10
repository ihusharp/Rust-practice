use std::{sync::{atomic::{AtomicBool, Ordering}}, cell::UnsafeCell, ops::{Deref, DerefMut}, thread};

pub struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub fn new(value: T) -> Self {
        Self { 
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }
    

    pub fn lock(&self) -> Guard<T> {
        while self.locked.swap(true, Ordering::Acquire) {
            std::hint::spin_loop();
        }
        // safely assume that the existence of a Guard 
        // means that the SpinLock has been locked.
        Guard { lock: self }
    }

    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

pub struct Guard<'a, T> {
    lock: &'a SpinLock<T>,
}

// make Guard<T> behave like an exclusive reference to T
impl<T> Deref for Guard<'_, T> {
    type Target = T;
    
    fn deref(&self) -> &T {
        // Safety: The very existence of this Guard
        // guarantees we've exclusively locked the lock.
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // Safety: The very existence of this Guard
        // guarantees we've exclusively locked the lock.
        unsafe { &mut *self.lock.value.get() }
    }
}

impl <T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

#[allow(dead_code)]
pub fn spinlock() {
    let spinlock = SpinLock::new(Vec::new());
    thread::scope(|s| {
        s.spawn(|| {
            spinlock.lock().push(1);
        });
        s.spawn(|| {
            let mut guard = spinlock.lock();
            guard.push(2);
            guard.push(3);
        });
    });

    let guard = spinlock.lock();
    assert!(guard.len() == 3);
}