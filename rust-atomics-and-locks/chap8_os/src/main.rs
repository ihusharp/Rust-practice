use std::{sync::atomic::{AtomicU32, Ordering}, thread, time::Duration};

fn main() {
    let a = AtomicU32::new(0);

    thread::scope(|s| {
        s.spawn(|| {
            thread::sleep(Duration::from_secs(3));
            if a.load(Ordering::Relaxed) == 2 {
                a.store(1, Ordering::Relaxed);
                wake_one(&a);
            } else {
                a.store(1, Ordering::Relaxed);
            }
        });

        println!("Waiting...");
        a.store(2, Ordering::Relaxed);
        while a.load(Ordering::Relaxed) == 2 {
            wait(&a, 2);
        }
        println!("Done!");
    })
}

#[cfg(not(target_os = "linux"))]
compile_error!("Linux only. Sorry!");

// The wait operation takes an argument which specifies the value 
// we expect the atomic variable to have and will refuse to block if it doesn’t match.
pub fn wait(a: &AtomicU32, expected: u32) {
    // refer to the futex(2) man page for more information
    unsafe {
        libc::syscall(
            libc::SYS_futex,
            a as *const AtomicU32 as *mut u32, // The atomic to operate on.
            libc::FUTEX_WAIT,
            expected,
            std::ptr::null::<libc::timespec>(),// no timeout
        );
    }
}

pub fn wake_one(a: &AtomicU32) {
    unsafe {
        libc::syscall(
            libc::SYS_futex,
            a as *const AtomicU32 as *mut u32,
            libc::FUTEX_WAKE,
            1, // The number of waiters to wake up
        );
    }
}
