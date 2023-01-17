use tokio::{io::{self, AsyncReadExt, AsyncWriteExt}, fs::File, net::{TcpListener, TcpStream}};

#[tokio::main]
async fn main() -> io::Result<()> {
    let socket = TcpStream::connect("127.0.0.1:6152").await?;
    let (mut rd, mut wr) = io::split(socket);

    // Write data to the socket
    tokio::spawn(async move {
        wr.write_all(b"hello world").await?;
        wr.write_all(b"husharp\n").await?;
        
        Ok::<_, io::Error>(())
    });

    let mut buf = vec![0; 128];
    loop {
        let n = rd.read(&mut buf).await?;

        if n == 0 {
            break;
        }

        // Print what we read
        println!("GOT = {:?}", &buf[..n]);
    }

    Ok(())
}

async fn read() -> io::Result<()> {
    let mut f = File::open("foo.txt").await?;
    let mut buffer = [0; 10];

    // read up to 10 bytes
    let n = f.read(&mut buffer[..]).await?;
    println!("The bytes: {:?}", &buffer[..n]);
    
    Ok(())
}

// write
async fn write() -> io::Result<()> {
    let mut file = File::create("foo.txt").await?;

    // write a byte
    file.write_all(b"somr bytes").await?;
    Ok(())
}