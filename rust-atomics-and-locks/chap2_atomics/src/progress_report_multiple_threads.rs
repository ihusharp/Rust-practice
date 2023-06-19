use std::{
    sync::atomic::{AtomicU64, AtomicUsize, Ordering::Relaxed},
    thread,
    time::{Duration, Instant},
};

#[allow(dead_code)]
pub fn progress_report_multiple_threads() {
    let num_done = &AtomicUsize::new(0);

    thread::scope(|s| {
        for t in 0..4 {
            // A background thread to process all 100 items
            s.spawn(move || {
                for i in 0..25 {
                    println!("Processing item {} in thread {}", i, t);
                    thread::sleep(std::time::Duration::from_millis(100));
                    num_done.fetch_add(1, Relaxed);
                }
            });
        }

        // The main thread to report progress
        loop {
            let n = num_done.load(Relaxed);
            if n == 100 {
                break;
            }
            println!("Working.. {n}/100 done");
            thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    println!("Done!");
}

#[allow(dead_code)]
pub fn statistics() {
    let num_done = &AtomicUsize::new(0);
    let total_time = &AtomicU64::new(0);
    let max_time = &AtomicU64::new(0);

    thread::scope(|s| {
        for t in 0..4 {
            // A background thread to process all 100 items
            s.spawn(move || {
                for i in 0..25 {
                    let start_time = Instant::now();
                    println!("Processing item {} in thread {}", i, t);
                    thread::sleep(std::time::Duration::from_millis(100));
                    let time_taken = start_time.elapsed().as_micros() as u64;
                    total_time.fetch_add(time_taken, Relaxed);
                    max_time.fetch_max(time_taken, Relaxed);
                    num_done.fetch_add(1, Relaxed);
                }
            });
        }

        // The main thread to report progress
        loop {
            let total_time = Duration::from_micros(total_time.load(Relaxed));
            let max_time = Duration::from_micros(max_time.load(Relaxed));
            let n = num_done.load(Relaxed);
            if n == 100 {
                break;
            }
            if n == 0 {
                println!("Working.. nothing done yet.");
            } else {
                println!(
                    "Working.. {n}/100 done, {:?} average, {:?} peak",
                    total_time / n as u32,
                    max_time,
                );
            }
            thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    println!("Done!");
}
