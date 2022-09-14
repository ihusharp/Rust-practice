/*
 * @Descripttion: 
 * @Author: HuSharp
 * @Date: 2022-09-13 22:19:18
 * @LastEditTime: 2022-09-13 22:37:51
 * @@Email: ihusharp@gmail.com
 */
use std::io::Cursor;

use bytes::BytesMut;
use mini_redis::{Frame, Result};
use tokio::{net::TcpStream, io::{AsyncReadExt, BufWriter, AsyncWriteExt, self}};

struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
    cursor: usize,
}

impl Connection {
    fn new(stream: TcpStream) -> Connection {
        Connection { 
            stream: BufWriter::new(stream),
            buffer: BytesMut::with_capacity(1024),
            cursor: 0,
        }
    }

    pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
        loop {
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            if self.buffer.len() == self.cursor {
                self.buffer.resize(self.cursor * 2, 0);
            }
            // read into the buf
            let n = self.stream.read(
                &mut self.buffer[self.cursor..]).await?;
            
            if n == 0 {
                if self.cursor == 0 {
                    return Ok(None);
                } else {
                    return Err("connection closed before frame was fully read".into());
                }
            } else {
                self.cursor += n;
            }

        }       
    }

    fn parse_frame(&self) -> Result<Option<Frame>> {
        // ...
        let mut buf = Cursor::new(&self.buffer[..]);

        match Frame::check(&mut buf) {
            Ok(_) => {
                let frame = Frame::parse(&mut buf).unwrap();
                Ok(Some(frame))
            }
            Err(mini_redis::frame::Error::Incomplete) => Ok(None),
            Err(e) => Err(e.into()),
        }

        Ok(None)
    }

    pub async fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
        match frame {
            Frame::Error(val) => {
                self.stream.write_u8(b'-').await?;
                self.stream.write_all(val.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
        }

        self.stream.flush().await;

        Ok(())
    }


}


#[tokio::main]
async fn main() {

}