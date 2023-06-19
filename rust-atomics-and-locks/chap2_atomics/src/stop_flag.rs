use std::{io::stdin, sync::atomic::AtomicBool, thread};

#[allow(dead_code)]
pub fn stop_flag() {
    static STOP: AtomicBool = AtomicBool::new(false);

    // Spawn a thread to work
    let background_thread = thread::spawn(|| {
        while !STOP.load(std::sync::atomic::Ordering::Relaxed) {
            println!("Background thread is working...");
            thread::sleep(std::time::Duration::from_millis(100));
        }
    });

    // Wait for user input
    for line in stdin().lines() {
        match line.unwrap().as_str() {
            "help" => println!("Help, stop"),
            "s" => break,
            cmd => println!("Unknown command: {}", cmd),
        }
    }

    // Stop the background thread
    STOP.store(true, std::sync::atomic::Ordering::Relaxed);

    // Wait for the background thread to finish
    background_thread.join().unwrap();
}
