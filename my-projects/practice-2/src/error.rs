/*
 * @Descripttion: 
 * @Author: HuSharp
 * @Date: 2022-08-28 22:25:59
 * @LastEditTime: 2022-08-29 12:48:44
 * @@Email: ihusharp@gmail.com
 */
use failure::Fail;
use std::io;

/// Error type for kvs.
#[derive(Fail, Debug)]
pub enum KvsError {
    /// IO error.
    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error),
    /// non-existent key error.
    #[fail(display = "Key not found")]
    KeyNotFound,
    /// Serialization or deserialization error.
    #[fail(display = "{}", _0)]
    Serde(#[cause] serde_json::Error),
    /// Not a valid command.
    #[fail(display = "Not a valid command")]
    NotValidType,
}

impl From<io::Error> for KvsError {
    fn from(error: io::Error) -> Self {
        KvsError::Io(error)
    }
}

impl From<serde_json::Error> for KvsError {
    fn from(err: serde_json::Error) -> KvsError {
        KvsError::Serde(err)
    }
}

/// Result type for kvs.
pub type Result<T> = std::result::Result<T, KvsError>;