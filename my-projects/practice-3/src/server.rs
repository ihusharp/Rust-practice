use std::{net::{ToSocketAddrs, TcpListener, TcpStream}, io::{BufReader, BufWriter, Write}};

use crate::{KvsEngine, Result, common::{Request, GetResponse, SetResponse}};
use log::{error, info, debug};
use serde_json::Deserializer;

/// The server of a key value store.
pub struct KvsServer<E: KvsEngine> {
    engine: E,
}

/// The server of a key value store.
impl<E: KvsEngine> KvsServer<E> {
    /// Creates a new `KvsServer` with the given engine.
    pub fn new(engine: E) -> Self {
        KvsServer { engine }
    }

    /// Run the server listening on the given address
    pub fn run(&mut self, addr: impl ToSocketAddrs) -> Result<()> {
        let listener = TcpListener::bind(addr)?;
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    self.handle_client(stream)?;
                }
                Err(e) => {
                    error!("failed to connect: {}", e);
                }
            }

        }
        Ok(())
    }

    fn handle_client(&mut self, tcp: TcpStream) -> Result<()> {
        let peer_addr = tcp.peer_addr()?;
        info!("connected to {}", peer_addr);
        let reader = BufReader::new(&tcp);
        let mut writer = BufWriter::new(&tcp);
        let req_reader = Deserializer::from_reader(reader).into_iter::<Request>();

        for req in req_reader {
            let req = req?;
            match req {
                Request::Get { key } => {
                    let resp = match self.engine.get(key) {
                        Ok(value) => GetResponse::Ok(value) ,
                        Err(e) =>  GetResponse::Err(e.to_string()) ,
                    };
                    serde_json::to_writer(&mut writer, &resp)?;
                    writer.flush()?;
                    debug!("Response sent to {}: {:?}", peer_addr, resp);
                }
                Request::Set { key, value } => {
                    let resp = match self.engine.set(key, value) {
                        Ok(_) => SetResponse::Ok(()) ,
                        Err(e) => { SetResponse::Err(e.to_string()) },
                    };
                    serde_json::to_writer(&mut writer, &resp)?;
                    writer.flush()?;
                    debug!("Response sent to {}: {:?}", peer_addr, resp);
                }
                Request::Remove { key } => {
                    let resp = match self.engine.remove(key) {
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

}
