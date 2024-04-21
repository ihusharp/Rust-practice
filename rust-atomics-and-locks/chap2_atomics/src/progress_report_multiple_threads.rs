use std::{
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

#[allow(dead_code)]
pub fn progress_report_multiple_threads() {
    // change num_done to refrence
    let num_done = &AtomicUsize::new(0);

    let main_thread = thread::current();
    thread::scope(|s| {
        for t in 0..4 {
            // A background thread to process 25 items
            s.spawn(move || {
                for i in 0..25 {
                    println!("Processing item {} in thread {}", t * 25 + i, t);
                    thread::sleep(Duration::from_millis(100));
                    // change store to fetch_add
                    num_done.fetch_add(1, Ordering::Relaxed);
                }
            });
            main_thread.unpark();
        }

        // The main thread to report progress
        loop {
            let n = num_done.load(Ordering::Relaxed);
            if n == 100 {
                break;
            }
            println!("Working.. {n}/100 done");
            thread::park_timeout(Duration::from_secs(1));
        }
    });

    println!("Done!");
}

#[allow(dead_code)]
pub fn statistics() {
    // change num_done to refrence
    let num_done = &AtomicUsize::new(0);
    let total_time = &AtomicU64::new(0);
    let max_time = &AtomicU64::new(0);

    let main_thread = thread::current();
    thread::scope(|s| {
        for t in 0..4 {
            // A background thread to process 25 items
            s.spawn(move || {
                for i in 0..25 {
                    let start_time = Instant::now();
                    println!("Processing item {} in thread {}", t * 25 + i, t);
                    thread::sleep(Duration::from_millis(100));
                    num_done.fetch_add(1, Ordering::Relaxed);

                    let elapsed = start_time.elapsed().as_millis() as u64;
                    total_time.fetch_add(elapsed, Ordering::Relaxed);
                    max_time.fetch_max(elapsed, Ordering::Relaxed);
                }
            });
            main_thread.unpark();
        }

        // The main thread to report progress
        loop {
            let n = num_done.load(Ordering::Relaxed);
            if n == 100 {
                break;
            }
            let total_time = total_time.load(Ordering::Relaxed);
            let max_time = max_time.load(Ordering::Relaxed);
            if n == 0 {
                println!("Working.. nothing done yet.");
            } else {
                println!(
                    "Working.. {n}/100 done, average time: {}ms, max time: {}ms",
                    total_time / n as u64,
                    max_time
                );
            }
            thread::park_timeout(Duration::from_millis(500));
        }
    });

    println!("Done!");
}
