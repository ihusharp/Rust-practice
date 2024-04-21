use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering::SeqCst},
    thread,
};

static X: AtomicBool = AtomicBool::new(false);
static Y: AtomicBool = AtomicBool::new(false);
static Z: AtomicU64 = AtomicU64::new(0);

fn write_x() {
    X.store(true, SeqCst); // 1
}

fn write_y() {
    Y.store(true, SeqCst); // 2
}

fn read_x_then_y() {
    while !X.load(SeqCst) {
        std::hint::spin_loop()
    }
    if Y.load(SeqCst) {
        // 3
        Z.fetch_add(1, SeqCst);
    }
}

fn read_y_then_x() {
    while !Y.load(SeqCst) {
        std::hint::spin_loop()
    }
    if X.load(SeqCst) {
        // 4
        Z.fetch_add(1, SeqCst);
    }
}

// if 3 is false, meant that 1 is true
// Which meant store X <- store Y
// so 4 must be true
#[allow(dead_code)]
pub fn seq() {
    let t1 = thread::spawn(move || {
        write_x();
    });

    let t2 = thread::spawn(move || {
        write_y();
    });

    let t3 = thread::spawn(move || {
        read_x_then_y();
    });

    let t4 = thread::spawn(move || {
        read_y_then_x();
    });

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();
    t4.join().unwrap();

    assert_ne!(Z.load(SeqCst), 0);
}
