use std::{sync::atomic::AtomicUsize, thread};

/// use thread park to wake up main thread

#[allow(dead_code)]
pub fn synchronization() {
    let num_done = AtomicUsize::new(0);

    let main_thread = thread::current();

    thread::scope(|s| {
        // A background thread to process all 100 items
        s.spawn(|| {
            for i in 0..100 {
                println!("Processing item {}", i);
                num_done.store(i + 1, std::sync::atomic::Ordering::Relaxed);
                main_thread.unpark();
            }
        });

        // A foreground thread to report progress
        loop {
            let n = num_done.load(std::sync::atomic::Ordering::Relaxed);
            if n == 100 {
                break;
            }
            println!("Working.. {n}/100 done");
            thread::park_timeout(std::time::Duration::from_secs(1));
        }
    })
}
