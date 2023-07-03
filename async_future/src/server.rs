use std::fs;
use async_std::net::TcpListener;
use async_std::prelude::*;
use async_std::io::{Read, Write};
use futures::StreamExt;
use std::time::Duration;
use async_std::task::{self, spawn};

pub const HELLO_HTML: &str = "/data/nvme0n1/husharp/proj/personal/Rust-practice/async_future/src/hello.html";
const NOT_FOUND_HTML: &str = "/data/nvme0n1/husharp/proj/personal/Rust-practice/async_future/src/404.html";

pub async fn server() {
    // Listen for incoming TCP connections on localhost port 7878
    let listener = TcpListener::bind("127.0.0.1:7878").await.unwrap();

    // Block forever, handling each request that arrives at this IP address
    listener.incoming().for_each_concurrent(1024, |stream| async move {
        let stream = stream.unwrap();
        spawn(handle_connection(stream));
    }).await;
}

async fn handle_connection(mut stream: impl Read + Write + Unpin) {
    // Read the first 1024 bytes of data from the stream
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).await.unwrap();

    let get = b"GET / HTTP/1.1\r\n";
    let sleep = b"GET /sleep HTTP/1.1\r\n";

    // Respond with greetings or a 404,
    // depending on the data in the request
    let (status_line, filename) = if buffer.starts_with(get) {
        ("HTTP/1.1 200 OK\r\n\r\n", HELLO_HTML)
    } else if buffer.starts_with(sleep) {
        task::sleep(Duration::from_secs(5)).await;
        ("HTTP/1.1 200 OK\r\n\r\n", HELLO_HTML)
    } else {
        ("HTTP/1.1 404 NOT FOUND\r\n\r\n", NOT_FOUND_HTML)
    };
    let contents = fs::read_to_string(filename).unwrap();

    // Write response back to the stream,
    // and flush the stream to ensure the response is sent back to the client
    let response = format!("{status_line}{contents}");
    stream.write(response.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();
}


// Mock Test
// #[async_std::test]
pub async fn test_handle_connection() {
    let input = b"GET / HTTP/1.1\r\n";
    let mut content = vec![0u8; 1024];
    content[..input.len()].copy_from_slice(input);
    let mut stream = MockTcpStream {
        read_data: content,
        write_data: vec![],
    };
    handle_connection(&mut stream).await;
    let mut buf = [0u8; 1024];
    stream.read(&mut buf).await.unwrap();

    let expected_response = format!("HTTP/1.1 200 OK\r\n\r\n{}", fs::read_to_string(HELLO_HTML).unwrap());
    assert!(stream.write_data.starts_with(expected_response.as_bytes()));
    println!("Test async_future::server::test_handle_connection() passed!");
}

struct MockTcpStream {
    read_data: Vec<u8>,
    write_data: Vec<u8>,
}

impl Read for MockTcpStream {
    fn poll_read(
                self: std::pin::Pin<&mut Self>,
                _cx: &mut task::Context<'_>,
                buf: &mut [u8],
            ) -> task::Poll<std::io::Result<usize>> {
        let size = std::cmp::min(self.read_data.len(), buf.len());
        buf[..size].copy_from_slice(&self.read_data[..size]);
        task::Poll::Ready(Ok(size))
    }
}

impl Write for MockTcpStream {
    fn poll_write(
                mut self: std::pin::Pin<&mut Self>,
                _cx: &mut task::Context<'_>,
                buf: &[u8],
            ) -> task::Poll<std::io::Result<usize>> {
        self.write_data = buf.to_vec();
        task::Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
                self: std::pin::Pin<&mut Self>,
                _cx: &mut task::Context<'_>,
            ) -> task::Poll<std::io::Result<()>> {
        task::Poll::Ready(Ok(()))
    }

    fn poll_close(
                self: std::pin::Pin<&mut Self>,
                _cx: &mut task::Context<'_>,
            ) -> task::Poll<std::io::Result<()>> {
        task::Poll::Ready(Ok(()))
    }
}

impl Unpin for MockTcpStream {}