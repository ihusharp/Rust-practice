use std::{marker::PhantomPinned, pin::Pin};

use futures::{Future, pin_mut};

/// 如果你实现了Unpin，Pin可以让你在Safe Rust下拿到&mut T，否则会把你在Safe Rust下钉住（也就是拿不到&mut T）
#[derive(Debug)]
struct Test {
    a: String,
    b: *const String,
    _marker: PhantomPinned,
}

impl Test {
    fn new(txt: &str) -> Self {
        Self {
            a: txt.to_string(),
            b: std::ptr::null(),
            _marker: PhantomPinned,
        }
    }

    fn init<'a>(self: Pin<&'a mut Self>) {
        let self_ptr : *const String = &self.a;
        let this = unsafe { self.get_unchecked_mut() };
        this.b = self_ptr;
    }

    fn a(&self) -> &str {
        &self.a
    }
    fn b(&self) -> &str {
        unsafe {
            &*self.b
        }
    }
}

pub fn pin_test() {
    let mut test1 = Test::new("Hello");
    let mut test1 = unsafe { Pin::new_unchecked(&mut test1) };
    Test::init(test1.as_mut());
    let mut test2 = Test::new("World");
    let mut test2 = unsafe { Pin::new_unchecked(&mut test2) };
    Test::init(test2.as_mut());

    println!("a: {}, b: {}", test1.a(), test1.b());
    // use swap
    // std::mem::swap(test1.get_mut(), test2.get_mut());
    println!("a: {}, b: {}", test1.a(), test1.b());
    // test1.a = "I've changed now!".to_string();
    println!("a: {}, b: {}", test2.a(), test2.b());    
}

pub fn unpin_future() {
    fn execute_unpin_future(_x: impl Future<Output = ()> + Unpin) { /* ... */ }

    // let fut = async { /* ... */ };
    // execute_unpin_future(fut); // error

    let fut = async { /* ... */ };
    let fut = Box::pin(fut);
    execute_unpin_future(fut); // ok

    let fut = async { /* ... */ };
    pin_mut!(fut);
    execute_unpin_future(fut); // ok

}