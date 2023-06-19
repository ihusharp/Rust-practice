use std::{
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
    thread,
};

#[allow(dead_code)]
pub fn progress_report() {
    let num_done = AtomicUsize::new(0);

    thread::scope(|s| {
        // A background thread to process all 100 items
        s.spawn(|| {
            for i in 0..100 {
                println!("Processing item {}", i);
                thread::sleep(std::time::Duration::from_millis(100));
                num_done.store(i + 1, Relaxed);
            }
        });

        // A foreground thread to report progress
        loop {
            let n = num_done.load(Relaxed);
            if n == 100 {
                break;
            }
            println!("Working.. {n}/100 done");
            thread::sleep(std::time::Duration::from_secs(1));
        }

        println!("Done!");
    })
}

// It might take up to one whole second for the main thread to know,
// introducing an unnecessary delay at the end.
