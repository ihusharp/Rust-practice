use std::{
    cell::UnsafeCell,
    mem::ManuallyDrop,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct ArcData<T> {
    // Number of `Arc's`
    data_ref_cnt: AtomicUsize,
    // Number of `Weak`s, plus one if there are any `Arc`s.
    alloc_ref_cnt: AtomicUsize,
    // The data. `None` if there's only weak pointers left.
    data: UnsafeCell<ManuallyDrop<T>>,
}

pub struct Arc<T> {
    ptr: NonNull<ArcData<T>>,
}

unsafe impl<T: Sync + Send> Send for Arc<T> {}
unsafe impl<T: Sync + Send> Sync for Arc<T> {}

pub struct Weak<T> {
    ptr: NonNull<ArcData<T>>,
}

unsafe impl<T: Send + Sync> Send for Weak<T> {}
unsafe impl<T: Send + Sync> Sync for Weak<T> {}

impl<T> Arc<T> {
    pub fn new(data: T) -> Self {
        Arc {
            // Box::leak to give up our exclusive ownership of this allocation
            ptr: NonNull::from(Box::leak(Box::new(ArcData {
                data_ref_cnt: AtomicUsize::new(1),
                alloc_ref_cnt: AtomicUsize::new(1),
                data: UnsafeCell::new(ManuallyDrop::new(data)),
            }))),
        }
    }

    fn data(&self) -> &ArcData<T> {
        unsafe { self.ptr.as_ref() }
    }

    // can only be called as Arc::get_mut(&mut a), and not as a.get_mut()
    // This is advisable for types that implement Deref,
    // to avoid ambiguity with a similarly named method on the underlying T.
    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        // Acquire matches Weak::drop's Release decrement, to make sure any
        // upgraded pointers are visible in the next data_ref_count.load.
        // `usize::MAX` to represent a special "locked" state of the weak pointer counter.
        if arc.data().alloc_ref_cnt.compare_exchange(
            1, usize::MAX, Ordering::Acquire, Ordering::Relaxed).is_err() {
            return None;
        }

        let is_unique = arc.data().data_ref_cnt.load(Ordering::Relaxed) == 1; 
        // Release matches Acquire increment in `downgrade`, to make sure any
        // changes to the data_ref_count that come after `downgrade` don't
        // change the is_unique result above.
        arc.data().alloc_ref_cnt.store(1, Ordering::Release);
        if !is_unique {
            return None;
        }
        // Acquire to match Arc::drop's Release decrement, to make sure nothing
        // else is accessing the data.
        fence(Ordering::Acquire);
        if arc.data().data_ref_cnt.load(Ordering::Relaxed) == 1 {
            // Safety: There's only one reference(Arc) to the data,
            // to which we have exclusive access and no weak pointers.
            unsafe { Some(&mut *arc.data().data.get()) }
        } else {
            None
        }
    }

    fn downgrade(arc: &Self) -> Weak<T> {
        let mut n = arc.data().alloc_ref_cnt.load(Ordering::Relaxed);
        loop {
            // have to check for the special usize::MAX value to see if the weak pointer counter is locked
            if n == usize::MAX {
                std::hint::spin_loop();
                n = arc.data().alloc_ref_cnt.load(Ordering::Relaxed);
                continue;
            }
            assert!(n <= usize::MAX / 2);
            if let Err(e) = arc.data().alloc_ref_cnt.compare_exchange(
                n,
                n + 1,
                Ordering::Acquire, /* synchronizes with the release-store in the get_mut function */
                Ordering::Relaxed,
            ) {
                n = e;
                continue;
            }
            return Weak { ptr: arc.ptr };
        
        }
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
            return Some(Arc { ptr: self.ptr });
        }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let ptr = self.data().data.get();
        // Safety: Since there's an Arc to the data,
        // the data exists and may be shared.
        unsafe { &*self.data().data.get() }
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
        if self.data().data_ref_cnt.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Arc count overflow");
        }
        Arc { ptr: self.ptr }
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
        if self.data().data_ref_cnt.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                ManuallyDrop::drop(&mut *self.data().data.get());
            }
            // Now that there's no `Arc<T>`s left,
            // drop the implicit weak pointer that represented all `Arc<T>`s.
            drop(Weak { ptr: self.ptr });
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
