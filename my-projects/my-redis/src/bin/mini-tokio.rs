use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::{Duration, Instant};
// A utility that allows us to implement a `std::task::Waker` without having to
// use `unsafe` code.
use futures::task::{self, ArcWake};
// Used as a channel to queue scheduled tasks.
use crossbeam::channel;
use tokio::spawn;

fn main() {
    let mut mini_tokio = MiniTokio::new();

    mini_tokio.spawn(async {
        // Spawn a task
        spawn(async {
            // Wait for a little bit of time so that "world" is printed after
            // "hello"
            delay(Duration::from_millis(100)).await;
            println!("world");
        });

        // Spawn a second task
        spawn(async {
            println!("hello");
        });

        // We haven't implemented executor shutdown, so force the process to exit.
        delay(Duration::from_millis(200)).await;
        std::process::exit(0);
    });
    // start executor
    mini_tokio.run();
}

struct MiniTokio {
    // when invoke wake(), will send task to scheduled
    scheduled: channel::Receiver<Arc<Task>>,    
    sender: channel::Sender<Arc<Task>>,
}

struct Task {
    // The `Mutex` is to make `Task` implement `Sync`. Only
    // one thread accesses `future` at any given time. The
    // `Mutex` is not required for correctness. Real Tokio
    // does not use a mutex here, but real Tokio has
    // more lines of code than can fit in a single tutorial
    // page.
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
    // make a exectuor to send task to scheduled
    executor: channel::Sender<Arc<Task>>,
}

// The standard library provides low-level, unsafe  APIs for defining wakers.
// Instead of writing unsafe code, we will use the helpers provided by the
// `futures` crate to define a waker that is able to schedule our `Task`
// structure.
impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let _ = arc_self.executor.send(arc_self.clone());
    }
}

impl Task {
    fn poll(self: Arc<Self>) {
        let waker = task::waker(self.clone());
        let mut cx = Context::from_waker(&waker);

        let mut future = self.future.try_lock().unwrap();

        // poll the future
        let _ = future.as_mut().poll(&mut cx);
    }

    fn spwan<F>(future: F, sender: &channel::Sender<Arc<Task>>)
        where
            F: Future<Output = ()> + Send + 'static,
    {
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
            executor: sender.clone(),
        });
        let _ = sender.send(task);
    }
}

impl MiniTokio {
    fn new() -> MiniTokio {
        let (tx, rx) = channel::unbounded();
        MiniTokio { 
            scheduled: rx,
            sender: tx,
        }
    }

    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Task::spwan(future, &self.sender.clone());
    }

    // implement a run method to recieve and run the tasks
    fn run(&mut self) {
        while let Ok(task) = self.scheduled.recv() {
            task.poll();
        }
    }
}

async fn delay(time: Duration) {
    struct Delay {
        when: Instant,
        // The waker to notify once the delay has completed. The waker must be
        // accessible by both the timer thread and the future so it is wrapped
        // with `Arc<Mutex<_>>`
        waker: Option<Arc<Mutex<Waker>>>,
    }
    
    impl Future for Delay {
        type Output = ();
    
        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>)
            -> Poll<()>
        {
            // firstly, if there is the first time the future is called, spawn the timer thread.
            if let Some(waker) = &self.waker {
                let mut waker = waker.lock().unwrap();
    
                // Check if the stored waker matches the current tasks waker.
                // This is necessary as the `Delay` future instance may move to
                // a different task between calls to `poll`. If this happens, the
                // waker contained by the given `Context` will differ and we
                // must update our stored waker to reflect this change.
                if !waker.will_wake(cx.waker()) {
                    *waker = cx.waker().clone();
                }
            } else {
                let when = self.when;
                let waker = Arc::new(Mutex::new(cx.waker().clone()));
                self.waker = Some(waker.clone());
    
                // this is the first waker
                thread::spawn(move || {
                    let now = Instant::now();
                    if now < when {
                        thread::sleep(when - now);
                    }
                    let waker = waker.lock().unwrap();
                    waker.wake_by_ref();
                });
            }
    
            if Instant::now() >= self.when {
                println!("Hello world");
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }

    // Create an instance of our `Delay` future.
    let future = Delay {
        when: Instant::now() + time,
        waker: None,
    };

    future.await;
}