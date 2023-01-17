/*
 * @Descripttion: 
 * @Author: HuSharp
 * @Date: 2022-08-15 20:39:25
 * @LastEditTime: 2022-08-28 22:30:21
 * @@Email: ihusharp@gmail.com
 */
#![deny(missing_docs)]
//! A simple key/value store.

pub use error::{KvsError, Result};
pub use kv::KvStore;

mod error;
mod kv;