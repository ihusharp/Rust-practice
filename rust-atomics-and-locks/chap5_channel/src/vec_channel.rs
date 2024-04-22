use std::{
    collections::VecDeque,
    sync::{Condvar, Mutex},
};

struct SimpleChannel<T> {
    queue: Mutex<VecDeque<T>>,
    item_ready: Condvar,
}

impl<T> SimpleChannel<T> {
    fn new() -> Self {
        SimpleChannel {
            queue: Mutex::new(VecDeque::new()),
            item_ready: Condvar::new(),
        }
    }

    fn send(&self, value: T) {
        self.queue.lock().unwrap().push_back(value);
        self.item_ready.notify_one();
    }

    fn recieve(&self) -> Option<T> {
        let mut queue = self.queue.lock().unwrap();
        loop {
            if let Some(m) = queue.pop_front() {
                return Some(m);
            }
            queue = self.item_ready.wait(queue).unwrap();
        }
    }
}

pub fn channel() {
    let channel = SimpleChannel::new();
    channel.send(1);
    channel.send(2);
    println!("{:?}", channel.recieve());
    println!("{:?}", channel.recieve());
    // will wait forever
    // channel.recieve();
}
