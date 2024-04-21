use std::{
    sync::atomic::{
        AtomicI32,
        Ordering::Relaxed,
    },
    thread,
};

#[allow(dead_code)]
pub fn releaxed() {
    thread::scope(|s| {
        s.spawn(a);

        s.spawn(b);
    });

    thread::scope(|s| {
        s.spawn(relaxed_a);

        s.spawn(relaxed_b);
    });
}

static X: AtomicI32 = AtomicI32::new(0);
static Y: AtomicI32 = AtomicI32::new(0);

fn a() {
    X.store(10, Relaxed);
    Y.store(20, Relaxed);
}

fn b() {
    let y = Y.load(Relaxed);
    let x = X.load(Relaxed);
    println!("{x}, {y}");
}

#[allow(dead_code)]
pub fn check_a_b() {
    let t1 = thread::spawn(a);
    let t2 = thread::spawn(b);

    t1.join().unwrap();
    t2.join().unwrap();
}

static RELAXED: AtomicI32 = AtomicI32::new(0);

fn relaxed_a() {
    RELAXED.fetch_add(5, Relaxed);
    RELAXED.fetch_add(10, Relaxed);
}

fn relaxed_b() {
    let a = RELAXED.load(Relaxed);
    let b = RELAXED.load(Relaxed);
    let c = RELAXED.load(Relaxed);
    let d = RELAXED.load(Relaxed);
    println!("{a} {b} {c} {d}");
}
