use std::{
    sync::atomic::{
        fence, AtomicBool,
        Ordering::{Acquire, Relaxed, Release},
    },
    thread,
    time::Duration,
};

static mut DATA: [u64; 10] = [0; 10];

const ATOMIC_FALSE: AtomicBool = AtomicBool::new(false);
static READY: [AtomicBool; 10] = [ATOMIC_FALSE; 10];

#[allow(dead_code)]
pub fn fence_check() {
    for i in 0..10 {
        thread::spawn(move || {
            println!("Thread {} is ready", i);
            unsafe {
                DATA[i] = i as u64;
            }
            READY[i].store(true, Release);
        });
    }
    thread::sleep(Duration::from_millis(100));
    let ready: [bool; 10] = std::array::from_fn(|i| READY[i].load(Relaxed));
    if ready.contains(&true) {
        fence(Acquire);
        for i in 0..10 {
            if ready[i] {
                println!("data{i} = {}", unsafe { DATA[i] });
            }
        }
    }
}
