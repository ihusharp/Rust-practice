use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Instant,
};

// 10个线程，每个线程100000次处理
const N_THREADS: usize = 10;
const N_TIMES: usize = 100000;

// R是待修改变量，SPIN_LOCK则是标记锁
static mut R: usize = 0;
static SPIN_LOCK: AtomicBool = AtomicBool::new(false);

#[allow(dead_code)]
pub fn perform() {
    for t in 1..=N_THREADS {
        unsafe {
            R = 0;
        }
        let start_of_spin = Instant::now();

        let handles = (0..N_THREADS)
            .map(|i| {
                thread::spawn(move || {
                    unsafe {
                        for j in i * N_TIMES..(i + 1) * N_TIMES {
                            // 用while循环来阻塞线程，swap来保证判断和修改的原子性，此处用了最宽松的Relaxed
                            while SPIN_LOCK.swap(true, Ordering::Relaxed) {
                                std::hint::spin_loop();
                            }
                            // 修改数据，本身并不是线程安全的
                            R += j;
                            // 把锁改回false让所有线程继续抢锁
                            SPIN_LOCK.store(false, Ordering::Relaxed);
                        }
                    }
                })
            })
            .collect::<Vec<_>>();

        for handle in handles {
            handle.join().unwrap();
        }

        let time_of_spin = start_of_spin.elapsed();

        let r = Arc::new(Mutex::new(0));

        let start_of_mutex = Instant::now();

        // 标准的多线程修改数据方法
        let handles = (0..N_THREADS)
            .map(|i| {
                let r = r.clone();
                thread::spawn(move || {
                    for j in i * N_TIMES..(i + 1) * N_TIMES {
                        *r.lock().unwrap() += j;
                    }
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            handle.join().unwrap();
        }

        let time_of_mutex = start_of_mutex.elapsed();
        println!(
            "{t:3}: R = {}, r = {}, spin: {time_of_spin:?}, mutex: {time_of_mutex:?}",
            unsafe { R },
            r.lock().unwrap()
        );
    }
}
