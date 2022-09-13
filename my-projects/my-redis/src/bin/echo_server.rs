use tokio::{io::{self, AsyncWriteExt, AsyncReadExt}, net::{TcpStream, TcpListener}};

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6152").await?;

    loop {
        let (mut socket, _) = listener.accept().await?;
        
        tokio::spawn(async move {
            let mut buf = vec![0; 128];
            loop {
                match socket.read(&mut buf).await {
                    // socket closed
                    Ok(0) => return,
                    Ok(n) => {
                        if socket.write_all(&buf[0..n]).await.is_err() {
                            return;
                        }
                    }
                    Err(_) => {
                        return;
                    }
                }
            }
        });
    }

}