use std::{thread, sync::{Arc, Mutex}, panic::{self, AssertUnwindSafe}};

use crossbeam::{Sender, Receiver};
use log::debug;

use crate::Result;

use super::ThreadPool;



// Note for Rust training course: the thread pool is not implemented using
// `catch_unwind` because it would require the task to be `UnwindSafe`.

/// A thread pool using a shared queue inside.
///
/// If a spawned task panics, the old thread will be destroyed and a new one will be
/// created. It fails silently when any failure to create the thread at the OS level
/// is captured after the thread pool is created. So, the thread number in the pool
/// can decrease to zero, then spawning a task to the thread pool will panic.
pub struct SharedQueueThreadPool {
    tx: Sender<Messsge>,
    workers: Vec<Worker>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Messsge {
    NewJob(Job),
    Shutdown,
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Drop for SharedQueueThreadPool {
    fn drop(&mut self) {
        debug!("Sending shutdown message to all workers");
        for _ in &self.workers {
            self.tx.send(Messsge::Shutdown).unwrap();
        }

        debug!("Shutting down {} workers", self.workers.len());

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

impl Worker {
    fn new(id: usize, rx: Arc<Mutex<Receiver<Messsge>>>) -> Self {
        let thread = thread::spawn(move || loop {
            let message = rx.lock().unwrap().recv().unwrap();
            match message {
                Messsge::NewJob(job) => {
                    if let Err(err) = panic::catch_unwind(AssertUnwindSafe(job)) {
                        debug!("Worker {} caught a panic: {:?}", id, err);
                    }
                }
                Messsge::Shutdown => {
                    debug!("Worker {} was told to terminate.", id);
                    break;
                }
            }
        });
        Worker {
            id,
            thread: Some(thread),
        }
    }
}

impl ThreadPool for SharedQueueThreadPool {
    fn new(threads: usize) -> Result<Self> 
    where
        Self: Sized,
    {
        let (tx, rx) = crossbeam::unbounded();
        let rx = Arc::new(Mutex::new(rx));
        let mut workers = Vec::new();
        for id in 0..threads {
            workers.push(Worker::new(id, Arc::clone(&rx)));
        }
        Ok(SharedQueueThreadPool { tx, workers })
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.tx
        .send(Messsge::NewJob(Box::new(job)))
        .expect("The thread pool has no thread.");
    }

}