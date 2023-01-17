/*
 * @Descripttion:
 * @Author: HuSharp
 * @Date: 2022-08-15 20:39:25
 * @LastEditTime: 2022-09-04 22:25:21
 * @@Email: ihusharp@gmail.com
 */
#![deny(missing_docs)]
//! A simple key/value store.

pub use client::KvsClient;
pub use engines::{KvStore, KvsEngine, SledKvStore};
pub use error::{KvsError, Result};
pub use server::KvsServer;

mod client;
mod common;
mod engines;
mod error;
mod server;