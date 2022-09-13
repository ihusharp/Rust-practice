use mini_redis::{client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Open a connection to the redis server listening on localhost:6379.
    let mut client = client::connect("127.0.0.1:6379").await.unwrap();

    client.set("hello", "world".into()).await?;

    let result = client.get("hello").await.unwrap();

    println!("get value from server, result = {:?}", result);    

    Ok(())
}