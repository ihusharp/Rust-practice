use std::{
    sync::atomic::{
        AtomicBool, AtomicPtr,
        Ordering::{Acquire, Relaxed, Release},
    },
    thread,
    time::Duration,
};

static mut DATA_U64: u64 = 0;
static READY: AtomicBool = AtomicBool::new(false);

#[allow(dead_code)]
pub fn acquire_and_release() {
    thread::spawn(|| {
        unsafe {
            DATA_U64 = 123;
        }
        READY.store(true, Release); // Everything from before this store ..
    });
    while !READY.load(Acquire) {
        // .. is visible after this loads `true`.
        thread::sleep(Duration::from_millis(100));
        println!("waiting...");
    }
    println!("{}", unsafe { DATA_U64 });
}

#[allow(dead_code)]
pub fn locking() {
    thread::scope(|s| {
        for _ in 0..100 {
            s.spawn(f_lock);
        }
    });
}

static mut DATA_STRING: String = String::new();
static LOCK: AtomicBool = AtomicBool::new(false);

fn f_lock() {
    if LOCK.compare_exchange(false, true, Acquire, Relaxed).is_ok() {
        unsafe {
            DATA_STRING.push_str("hello");
        }
        LOCK.store(false, Release);
    }

    if LOCK.swap(true, Acquire) == false {
        unsafe {
            DATA_STRING.push_str("world");
        }
        LOCK.store(false, Release);
    }
}

#[allow(dead_code)]
pub struct Data {
    a: u64,
}

#[allow(dead_code)]
pub fn get_data() -> &'static Data {
    static PTR: AtomicPtr<Data> = AtomicPtr::new(std::ptr::null_mut());

    let mut p = PTR.load(Acquire);
    if p.is_null() {
        p = Box::into_raw(Box::new(Data { a: 100 }));
        if let Err(e) = PTR.compare_exchange(std::ptr::null_mut(), p, Release, Acquire) {
            drop(unsafe { Box::from_raw(p) });
            p = e;
        }
    }

    unsafe { &*p }
}
