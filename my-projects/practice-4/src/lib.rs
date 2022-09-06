/*
 * @Descripttion:
 * @Author: HuSharp
 * @Date: 2022-08-15 20:39:25
 * @LastEditTime: 2022-09-06 15:37:16
 * @@Email: ihusharp@gmail.com
 */
#![deny(missing_docs)]
//! A simple key/value store.

pub use client::KvsClient;
pub use engines::{KvStore, KvsEngine};
pub use error::{KvsError, Result};
pub use server::KvsServer;

mod client;
mod common;
mod engines;
mod error;
mod server;
pub mod thread_pool;