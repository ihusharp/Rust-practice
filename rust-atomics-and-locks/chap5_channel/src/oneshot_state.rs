use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicU8, Ordering},
    thread,
};

const EMPTY: u8 = 0;
const WRITING: u8 = 1;
const READY: u8 = 2;
const READING: u8 = 3;

struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    state: AtomicU8,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Channel<T> {
    fn new() -> Self {
        Channel {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            state: AtomicU8::new(EMPTY),
        }
    }

    /// can send only once!!!
    fn send(&self, message: T) {
        if self
            .state
            .compare_exchange(EMPTY, WRITING, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            panic!("channel already used!");
        }
        unsafe {
            (*self.message.get()).write(message);
        }
        self.state.store(READY, Ordering::Release);
    }

    fn is_ready(&self) -> bool {
        self.state.load(Ordering::Relaxed) == READY
    }

    /// Panics if no message is available yet.
    /// or if called more than once.
    ///
    /// Tip: Use `is_ready` to check first.
    ///
    /// Safety: Only call this once!
    fn receive(&self) -> T {
        if self
            .state
            .compare_exchange(READY, READING, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            panic!("no message available!");
        }
        unsafe { (*self.message.get()).assume_init_read() }
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        // takes an exclusive reference to avoid write conflicts
        if *self.state.get_mut() == READY {
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
