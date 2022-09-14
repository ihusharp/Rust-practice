// wait for a specify time
use std::{time::{Instant, Duration}, future::Future, task::Poll, pin::Pin};


struct Delay {
    when: Instant,
}

impl Future for Delay {
    type Output = &'static str;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if Instant::now() >= self.when {
            println!("Hello world!");
            Poll::Ready("done")
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

// implement Delay to a state machine
enum MainFuture {
    // Initial state
    Start,
    // Waiting for the delay to complete
    Delay(Delay),
    // Delay has completed
    Done,
}

impl Future for MainFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<()> {
        loop {
            match *self {
                MainFuture::Start => {
                    let when = Instant::now() + Duration::from_secs(1);
                    let delay = Delay{ when };
                    *self = MainFuture::Delay(delay);
                }
                MainFuture::Delay(ref mut delay) => {
                    match Pin::new(delay).poll(cx) {
                        Poll::Ready(out) => {
                            assert_eq!(out, "done");
                            *self = MainFuture::Done;
                            return Poll::Ready(());
                        }
                        Poll::Pending => return Poll::Pending,
                    }
                }
                MainFuture::Done => {
                    panic!("future polled after completion");
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let when = Instant::now() + Duration::from_millis(10);
    let future = Delay { when };

    let out = future.await;
    assert_eq!(out, "done");
}