use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
    thread::{self, Thread},
};

struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

struct Sender<'a, T> {
    channel: &'a Channel<T>,
    receiving_thread: Thread,
}
struct Receiver<'a, T> {
    channel: &'a Channel<T>,
    _no_send: PhantomData<*const ()>,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Channel<T> {
    fn new() -> Self {
        Channel {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }

    fn split(&mut self) -> (Sender<T>, Receiver<T>) {
        // This `*self` invokes the Drop implementation on the old *self
        // which can avoid call `split` twice
        *self = Channel::new();
        (
            Sender {
                channel: self,
                receiving_thread: thread::current(),
            },
            Receiver {
                channel: self,
                // make reciever not send, stay on current thread
                _no_send: PhantomData,
            },
        )
    }
}

/// they cannot end up with multiple copies of either of them,
/// guaranteeing that send and receive can each only be called once.
impl<T> Sender<'_, T> {
    /// can send only once!!!
    fn send(self, message: T) {
        unsafe {
            (*self.channel.message.get()).write(message);
        }
        self.channel.ready.store(true, Ordering::Release);
        self.receiving_thread.unpark();
    }
}

impl<T> Receiver<'_, T> {
    /// wait until message is ready
    ///
    /// Tip: Use `is_ready` to check first.
    ///
    /// Safety: Only call this once!
    fn receive(self) -> T {
        // user might call `recieve` before `is_ready`
        // something other than our send method called unpark()
        // so we need to check if message is ready by `while`.
        while !self.channel.ready.swap(false, Ordering::Acquire) {
            thread::park();
        }
        unsafe { (*self.channel.message.get()).assume_init_read() }
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
    let mut channel = Channel::new();
    let t = thread::current();
    thread::scope(|s| {
        let (sender, receiver) = Channel::split(&mut channel);
        s.spawn(|| {
            sender.send("hello world!");
            t.unpark();
        });
        // if channel never receives a message, we can drop channel well
        assert_eq!(receiver.receive(), "hello world!");
    });
}
