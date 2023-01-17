/*
 * @Descripttion: 
 * @Author: HuSharp
 * @Date: 2022-09-04 16:07:06
 * @LastEditTime: 2022-09-04 17:50:13
 * @@Email: ihusharp@gmail.com
 */
use std::{
    io::{BufReader, BufWriter, Write},
    net::{TcpStream, ToSocketAddrs},
};

use serde::Deserialize;
use serde_json::{de::IoRead, Deserializer};

use crate::{
    common::{GetResponse, RemoveResponse, SetResponse, Request},
    KvsError, Result,
};

/// implements the functionality required for kvs-client to speak to kvs-server
pub struct KvsClient {
    reader: Deserializer<IoRead<BufReader<TcpStream>>>,
    writer: BufWriter<TcpStream>,
}

impl KvsClient {
    /// Connect to `addr` to access `KvsServer`.
    pub fn connect<A: ToSocketAddrs>(addr: A) -> Result<KvsClient> {
        let tcp_reader = TcpStream::connect(addr)?;
        let tcp_writer = tcp_reader.try_clone()?;
        Ok(KvsClient {
            reader: Deserializer::from_reader(BufReader::new(tcp_reader)),
            writer: BufWriter::new(tcp_writer),
        })
    }

    /// Get the value of a given key from the server.
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        serde_json::to_writer(&mut self.writer, &Request::Get { key })?;
        self.writer.flush()?;
        let response = GetResponse::deserialize(&mut self.reader)?;
        match response {
            GetResponse::Ok(value) => Ok(value),
            GetResponse::Err(err) => Err(KvsError::StringError(err)),
        }
    }

    /// Set the value of a given key from the server.
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        serde_json::to_writer(&mut self.writer, &Request::Set { key, value })?;
        self.writer.flush()?;
        let response = SetResponse::deserialize(&mut self.reader)?;
        match response {
            SetResponse::Ok(_) => Ok(()),
            SetResponse::Err(err) => Err(KvsError::StringError(err)),
        }
    }

    /// Remove a given key from the server.
    pub fn remove(&mut self, key: String) -> Result<()> {
        serde_json::to_writer(&mut self.writer, &Request::Remove { key })?;
        self.writer.flush()?;
        let response = RemoveResponse::deserialize(&mut self.reader)?;
        match response {
            RemoveResponse::Ok(_) => Ok(()),
            RemoveResponse::Err(err) => Err(KvsError::StringError(err)),
        }
    }
}
