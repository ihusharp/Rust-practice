use std::{
    collections::VecDeque,
    thread,
    time::{Duration, Instant},
};

use mutex_without_syscall_spin::Mutex;

// use mutex_syscall::Mutex;
// use mutex_without_syscall::Mutex;
// use mutex_without_syscall_spin::Mutex;

mod mutex_syscall;
mod mutex_without_syscall;
mod mutex_without_syscall_spin;

mod condvar;
mod condvar_without_syscall;

mod rwlock;
mod rwlock_avoid_busy;
mod rwlock_avoid_write_starvation;

fn main() {
    // check for mutex
    let m = Mutex::new(0);
    std::hint::black_box(&m);
    let start = Instant::now();
    thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| {
                for _ in 0..5_000_000 {
                    *m.lock() += 1;
                }
            });
        }
    });
    let duration = start.elapsed();
    println!("locked {} times in {:?}", *m.lock(), duration);
}
