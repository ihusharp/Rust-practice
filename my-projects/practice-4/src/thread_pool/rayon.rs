use std::thread;

use crate::Result;

use super::ThreadPool;

/// It is actually not a thread pool. It spawns a new thread every time
/// the `spawn` method is called.
pub struct RayonThreadPool;

impl ThreadPool for RayonThreadPool {
    fn new(_threads: u32) -> Result<Self> {
        Ok(RayonThreadPool)
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        thread::spawn(job);
    }
}