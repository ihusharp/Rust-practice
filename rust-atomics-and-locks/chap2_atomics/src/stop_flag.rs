use std::{
    io::stdin,
    sync::atomic::{AtomicBool, Ordering},
    thread::{self, sleep},
    time::Duration,
};

#[allow(dead_code)]
pub fn stop_flag() {
    static STOP: AtomicBool = AtomicBool::new(false);

    let background_thread = thread::spawn(|| {
        while !STOP.load(Ordering::Relaxed) {
            println!("Background thread is running");
            sleep(Duration::from_secs(1))
        }
    });

    // Use the main thread to listen for user input
    for line in stdin().lines() {
        match line.unwrap().as_str() {
            "stop" => {
                STOP.store(true, Ordering::Relaxed);
                break;
            }
            _ => println!("Unknown command"),
        }
    }

    STOP.store(true, Ordering::Relaxed);

    background_thread.join().unwrap();
}
