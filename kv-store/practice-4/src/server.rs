/*
 * @Descripttion: 
 * @Author: HuSharp
 * @Date: 2022-09-05 12:05:39
 * @LastEditTime: 2022-09-06 16:07:06
 * @@Email: ihusharp@gmail.com
 */
use std::{net::{ToSocketAddrs, TcpListener, TcpStream}, io::{BufReader, BufWriter, Write}};

use crate::{KvsEngine, Result, common::{Request, GetResponse, SetResponse}, thread_pool::ThreadPool};
use log::{error, info, debug};
use serde_json::Deserializer;

/// The server of a key value store.
pub struct KvsServer<E: KvsEngine, P: ThreadPool> {
    engine: E,
    pool: P,
}

/// The server of a key value store.
impl<E: KvsEngine, P: ThreadPool> KvsServer<E, P> {
    /// Creates a new `KvsServer` with the given engine.
    pub fn new(engine: E, pool: P) -> Self {
        KvsServer { engine, pool }
    }

    /// Run the server listening on the given address
    pub fn run(&mut self, addr: impl ToSocketAddrs) -> Result<()> {
        let listener = TcpListener::bind(addr)?;
        for stream in listener.incoming() {
            let engine = self.engine.clone();            
            self.pool.spawn(move || match stream {
               Ok(stream) => {
                    if let Err(e) = handle_client(engine, stream) {
                        error!("Error handling client: {}", e);
                    }
               } 
               Err(e) => error!("failed to accept connection: {}", e),
            });
        }
        Ok(())
    }
}

fn handle_client<E: KvsEngine>(engine: E, tcp: TcpStream) -> Result<()> {
    let peer_addr = tcp.peer_addr()?;
    info!("connected to {}", peer_addr);
    let reader = BufReader::new(&tcp);
    let mut writer = BufWriter::new(&tcp);
    let req_reader = Deserializer::from_reader(reader).into_iter::<Request>();

    for req in req_reader {
        let req = req?;
        match req {
            Request::Get { key } => {
                let resp = match engine.get(key) {
                    Ok(value) => GetResponse::Ok(value) ,
                    Err(e) =>  GetResponse::Err(e.to_string()) ,
                };
                serde_json::to_writer(&mut writer, &resp)?;
                writer.flush()?;
                debug!("Response sent to {}: {:?}", peer_addr, resp);
            }
            Request::Set { key, value } => {
                let resp = match engine.set(key, value) {
                    Ok(_) => SetResponse::Ok(()) ,
                    Err(e) => { SetResponse::Err(e.to_string()) },
                };
                serde_json::to_writer(&mut writer, &resp)?;
                writer.flush()?;
                debug!("Response sent to {}: {:?}", peer_addr, resp);
            }
            Request::Remove { key } => {
                let resp = match engine.remove(key) {
                    Ok(_) => SetResponse::Ok(()) ,
                    Err(e) => { SetResponse::Err(e.to_string()) },
                };
                serde_json::to_writer(&mut writer, &resp)?;
                writer.flush()?;
                debug!("Response sent to {}: {:?}", peer_addr, resp);
            }
        }
    }

    Ok(())
}

