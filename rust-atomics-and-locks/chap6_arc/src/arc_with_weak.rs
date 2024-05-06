use std::{
    cell::UnsafeCell,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct ArcData<T> {
    // Number of `Arc's`
    data_ref_cnt: AtomicUsize,
    // Number of `Weak's` and `Arc's` combined
    alloc_ref_cnt: AtomicUsize,
    // The data. `None` if there's only weak pointers left.
    data: UnsafeCell<Option<T>>,
}

pub struct Arc<T> {
    weak: Weak<T>,
}

pub struct Weak<T> {
    ptr: NonNull<ArcData<T>>,
}

unsafe impl<T: Send + Sync> Send for Arc<T> {}
unsafe impl<T: Send + Sync> Sync for Arc<T> {}

unsafe impl<T: Send + Sync> Send for Weak<T> {}
unsafe impl<T: Send + Sync> Sync for Weak<T> {}

impl<T> Arc<T> {
    pub fn new(data: T) -> Self {
        Arc {
            weak: Weak {
                // Box::leak to give up our exclusive ownership of this allocation
                ptr: NonNull::from(Box::leak(Box::new(ArcData {
                    data_ref_cnt: AtomicUsize::new(1),
                    alloc_ref_cnt: AtomicUsize::new(1),
                    data: UnsafeCell::new(Some(data)),
                }))),
            },
        }
    }

    // can only be called as Arc::get_mut(&mut a), and not as a.get_mut()
    // This is advisable for types that implement Deref,
    // to avoid ambiguity with a similarly named method on the underlying T.
    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        // because weak<T> can be upgraded to Arc<T> by upgrade()
        if arc.weak.data().alloc_ref_cnt.load(Ordering::Relaxed) == 1 {
            fence(Ordering::Acquire);
            // Safety: There's only one reference(Arc) to the data,
            // to which we have exclusive access and no weak pointers.
            let arc_data = unsafe { arc.weak.ptr.as_mut() };
            arc_data.data.get_mut().as_mut()
        } else {
            None
        }
    }

    fn downgrade(arc: &Self) -> Weak<T> {
        arc.weak.clone()
    }
}

impl<T> Weak<T> {
    fn data(&self) -> &ArcData<T> {
        unsafe { self.ptr.as_ref() }
    }

    // Upgrading a Weak to an Arc is only possible when the data still exists.
    pub fn upgrade(&self) -> Option<Arc<T>> {
        let mut n = self.data().data_ref_cnt.load(Ordering::Relaxed);
        // use a compare-and-swap loop to increment the data reference counter
        loop {
            if n == 0 {
                return None;
            }
            assert!(n <= usize::MAX / 2);
            if let Err(e) = self.data().data_ref_cnt.compare_exchange(
                n,
                n + 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                n = e;
                continue;
            }
            return Some(Arc { weak: self.clone() });
        }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let ptr = self.weak.data().data.get();
        // Safety: Since there's an Arc to the data,
        // the data exists and may be shared.
        unsafe { (*ptr).as_ref().unwrap() }
    }
}

impl<T> Clone for Weak<T> {
    fn clone(&self) -> Self {
        if self.data().alloc_ref_cnt.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Weak count overflow");
        }
        Weak { ptr: self.ptr }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        let weak = self.weak.clone();
        if weak.data().data_ref_cnt.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Arc count overflow");
        }
        Arc { weak }
    }
}

impl<T> Drop for Weak<T> {
    fn drop(&mut self) {
        // we can’t use Relaxed ordering, since we need to
        // make sure that nothing is still accessing the data when we drop it.
        if self.data().alloc_ref_cnt.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                drop(Box::from_raw(self.ptr.as_ptr()));
            }
        }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        // will drop weak simultaneously
        if self
            .weak
            .data()
            .data_ref_cnt
            .fetch_sub(1, Ordering::Release)
            == 1
        {
            fence(Ordering::Acquire);
            let ptr = self.weak.data().data.get();
            // Safety: The data reference counter is zero,
            // so nothing will access it.
            unsafe {
                (*ptr) = None;
            }
        }
    }
}

#[test]
fn test_arc() {
    static NUM_DROPS: AtomicUsize = AtomicUsize::new(0);

    struct DetectDrop;

    impl Drop for DetectDrop {
        fn drop(&mut self) {
            NUM_DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }

    // Create two Arcs sharing an object containing a string
    // and a DetectDrop, to detect when it's dropped.
    let x = Arc::new(("hello", DetectDrop));
    let y = x.clone();

    // Send x to a new thread, and wait for it to be dropped.
    let t = std::thread::spawn(move || {
        assert_eq!(x.0, "hello");
    });

    // In parallel, y should still be alive.
    assert_eq!(y.0, "hello");

    // Wait for the thread to finish.
    t.join().unwrap();

    // Check now only one copy of the object is alive.
    assert_eq!(NUM_DROPS.load(Ordering::Relaxed), 0);

    // Drop the last copy.
    drop(y);

    // Check the object was dropped.
    assert_eq!(NUM_DROPS.load(Ordering::Relaxed), 1);
}

#[test]
fn test_weak() {
    static NUM_DROPS: AtomicUsize = AtomicUsize::new(0);

    struct DetectDrop;

    impl Drop for DetectDrop {
        fn drop(&mut self) {
            NUM_DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }

    // Create an Arc with two weak pointers to it.
    let x = Arc::new(("hello", DetectDrop));
    let weak1 = Arc::downgrade(&x);
    let weak2 = Arc::downgrade(&x);

    let t = std::thread::spawn(move || {
        // Weak pointer should be upgradable at this point.
        let y = weak1.upgrade().unwrap();
        assert_eq!(y.0, "hello");
    });
    assert_eq!(x.0, "hello");
    t.join().unwrap();

    // The data shouldn't be dropped yet,
    assert_eq!(NUM_DROPS.load(Ordering::Relaxed), 0);
    // and the weak pointer should be upgradable.
    assert!(weak2.upgrade().is_some());

    drop(x);

    // The data should be dropped now.
    assert_eq!(NUM_DROPS.load(Ordering::Relaxed), 1);
    // The weak pointer should no longer be upgradable.
    assert!(weak2.upgrade().is_none());
}
