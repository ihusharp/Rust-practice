use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

struct Sender<T> {
    channel: Arc<Channel<T>>,
}
struct Receiver<T> {
    channel: Arc<Channel<T>>,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Channel<T> {
    fn channel() -> (Sender<T>, Receiver<T>) {
        let channel = Arc::new(Channel {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        });
        (
            Sender {
                channel: channel.clone(),
            },
            Receiver { channel },
        )
    }
}

/// they cannot end up with multiple copies of either of them,
/// guaranteeing that send and receive can each only be called once.
impl<T> Sender<T> {
    /// can send only once!!!
    fn send(self, message: T) {
        unsafe {
            (*self.channel.message.get()).write(message);
        }
        self.channel.ready.store(true, Ordering::Release);
    }
}

impl<T> Receiver<T> {
    fn is_ready(&self) -> bool {
        self.channel.ready.load(Ordering::Relaxed)
    }

    /// Panics if no message is available yet.
    ///
    /// Tip: Use `is_ready` to check first.
    ///
    /// Safety: Only call this once!
    fn receive(self) -> T {
        // still need to be panic because user might call `recieve` before `is_ready`
        if !self.channel.ready.swap(false, Ordering::Acquire) {
            panic!("no message available!");
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
    let (sender, receiver) = Channel::channel();
    let t = thread::current();
    thread::scope(|s| {
        s.spawn(|| {
            sender.send("hello world!");
            t.unpark();
        });
        while !receiver.is_ready() {
            thread::park();
        }
        // if channel never receives a message, we can drop channel well
        assert_eq!(receiver.receive(), "hello world!");
    });
}
