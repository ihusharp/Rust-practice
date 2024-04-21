#[test]
fn test_concurrent_relaxed_memory_ordering() {
    use loom::sync::atomic::Ordering::Relaxed;
    use loom::sync::atomic::AtomicU32;
    use loom::thread;

    loom::model(|| {
        loom::lazy_static! {
            static ref X: AtomicU32 = AtomicU32::new(0);
        }
        let t1 = thread::spawn(|| {
            X.fetch_add(5, Relaxed);
            X.fetch_add(10, Relaxed);
        });

        let t2 = thread::spawn(|| {
            let a = X.load(Relaxed);
            let b = X.load(Relaxed);
            let c = X.load(Relaxed);
            let d = X.load(Relaxed);
            assert!(a == 0 || a == 5 || a == 15, "a = {}", a);
            assert!(b == 0 || b == 5 || b == 15, "b = {}", a);
            assert!(c == 0 || c == 5 || c == 15, "c = {}", a);
            assert!(d == 0 || d == 5 || d == 15, "d = {}", a);

            assert!(d >= c || d >= b || d >= a, "d err");
            assert!(c >= a || c >= b || d >= c, "d err");
            assert!(b >= a || c >= b || d >= b, "b err");
            assert!(b >= a || c >= a || d >= a, "a err");
            println!("{a} {b} {c} {d}");
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });
}
