use std::{
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::Duration,
};

/// use thread park to wake up main thread immediately
#[allow(dead_code)]
pub fn synchronization() {
    let num_done = AtomicUsize::new(0);

    let main_thread = thread::current();
    thread::scope(|s| {
        s.spawn(|| {
            for i in 0..100 {
                println!("Processing item {}", i);
                thread::sleep(Duration::from_millis(100));
                num_done.store(i + 1, Ordering::Relaxed);
                main_thread.unpark();
            }
        });

        loop {
            let n = num_done.load(Ordering::Relaxed);
            if n == 100 {
                break;
            }
            println!("Working... {}/100 done", n);
            thread::park_timeout(Duration::from_secs(1));
        }
    })
}
