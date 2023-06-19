use std::{sync::{Mutex, Condvar}, collections::VecDeque, thread, time::Duration};

fn main() {
    let q = Mutex::new(VecDeque::new());
    let not_empty = Condvar::new();

    thread::scope(|s| {
        // Consuming the queue
        s.spawn(|| loop {
            let mut q = q.lock().unwrap();
            let item = loop {
                if let Some(item) = q.pop_front() {
                    break item;
                } else {
                    q = not_empty.wait(q).unwrap();
                }
            };
            drop(q);
            println!("1 Got: {}", item);
        });

        s.spawn(|| loop {
            let mut q = q.lock().unwrap();
            let item = loop {
                if let Some(item) = q.pop_front() {
                    break item;
                } else {
                    q = not_empty.wait(q).unwrap();
                }
            };
            drop(q);
            println!("2 Got: {}", item);
        });

        // Producing the queue
        for i in 0.. {
            q.lock().unwrap().push_back(i);
            not_empty.notify_one();
            thread::sleep(Duration::from_millis(100));
        }
    });
}