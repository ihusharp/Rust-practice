use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use rand::Rng;

#[allow(dead_code)]
pub fn id_allocation() -> u32 {
    static NEXT_ID: AtomicU32 = AtomicU32::new(0);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    if id >= 1000 {
        NEXT_ID.fetch_sub(1, Ordering::Relaxed);
        panic!("No more IDs available");
    }
    id
}

#[allow(dead_code)]
pub fn allocate_new_id() -> u32 {
    static NEXT_ID: AtomicU32 = AtomicU32::new(0);
    let mut id = NEXT_ID.load(Ordering::Relaxed);
    loop {
        assert!(id < 1000, "No more IDs available");
        match NEXT_ID.compare_exchange_weak(id, id + 1, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return id,
            Err(x) => id = x,
        }
    }

    // convenience method which called fetch_update
    // NEXT_ID.fetch_update(Relaxed, Relaxed, |n| n.checked_add(1)).expect("too many IDs!")
}

#[allow(dead_code)]
fn get_x_1() -> u64 {
    static X: AtomicU64 = AtomicU64::new(0);
    let mut x = X.load(Ordering::Relaxed);
    if x == 0 {
        x = generate_random_key();
        X.store(x, Ordering::Relaxed);
    }
    x
}

#[allow(dead_code)]
pub fn get_key() -> u64 {
    static KEY: AtomicU64 = AtomicU64::new(0);
    let key: u64 = KEY.load(Ordering::Relaxed);
    if key == 0 {
        let new_key = generate_random_key();
        println!("Generated new key: {}", new_key);
        match KEY.compare_exchange_weak(key, new_key, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => new_key,
            Err(x) => x,
        }
    } else {
        key
    }
}

#[allow(dead_code)]
fn generate_random_key() -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen::<u64>()
}
