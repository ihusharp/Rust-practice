use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
    thread,
};

struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    is_use: AtomicBool,
    ready: AtomicBool,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Channel<T> {
    fn new() -> Self {
        Channel {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            is_use: AtomicBool::new(false),
            ready: AtomicBool::new(false),
        }
    }

    /// can send only once!!!
    fn send(&self, message: T) {
        if self.is_use.swap(true, Ordering::Acquire) {
            panic!("channel already used!");
        }
        unsafe {
            (*self.message.get()).write(message);
        }
        self.ready.store(true, Ordering::Release);
    }

    fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    /// Panics if no message is available yet.
    /// or if called more than once.
    ///
    /// Tip: Use `is_ready` to check first.
    ///
    /// Safety: Only call this once!
    fn receive(&self) -> T {
        if !self.ready.swap(false, Ordering::Acquire) {
            panic!("no message available!");
        }
        unsafe { (*self.message.get()).assume_init_read() }
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        // takes an exclusive reference to avoid write conflicts
        if *self.ready.get_mut() {
            unsafe {
                (*self.message.get_mut()).assume_init_drop();
            }
        }
    }
}

pub fn oneshot_channel() {
    let channel = Channel::new();
    let t = thread::current();
    thread::scope(|s| {
        s.spawn(|| {
            channel.send("hello world!");
            t.unpark();
        });
        while !channel.is_ready() {
            thread::park();
        }
        // if channel never receives a message, we can drop channel well
        assert_eq!(channel.receive(), "hello world!");
    });
}
