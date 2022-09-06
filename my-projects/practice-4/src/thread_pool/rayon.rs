/*
 * @Descripttion: 
 * @Author: HuSharp
 * @Date: 2022-09-06 08:56:27
 * @LastEditTime: 2022-09-06 17:48:29
 * @@Email: ihusharp@gmail.com
 */
use std::thread;

use crate::Result;

use super::ThreadPool;

/// It is actually not a thread pool. It spawns a new thread every time
/// the `spawn` method is called.
pub struct RayonThreadPool;

impl ThreadPool for RayonThreadPool {
    fn new(_threads: usize) -> Result<Self> {
        Ok(RayonThreadPool)
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        thread::spawn(job);
    }
}