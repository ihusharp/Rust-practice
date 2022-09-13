/*
 * @Descripttion: 
 * @Author: HuSharp
 * @Date: 2022-09-13 10:47:51
 * @LastEditTime: 2022-09-13 20:45:37
 * @@Email: ihusharp@gmail.com
 */
use std::{collections::{HashMap, hash_map::DefaultHasher}, sync::{Arc, Mutex}, hash, ptr::hash};

use bytes::Bytes;
use mini_redis::{Connection, Frame};
use tokio::net::{TcpListener, TcpStream};

type Db = Arc<Mutex<HashMap<String, Bytes>>>;
type ShardedDb = Arc<Vec<Mutex<HashMap<String, Vec<u8>>>>>;

fn new_shared_db(num_shareds: usize) -> ShardedDb {
    let mut db = Vec::with_capacity(num_shareds);
    for _ in 0..num_shareds {
        db.push(Mutex::new(HashMap::new()));
    }
    Arc::new(db)
}

#[tokio::main]
async fn main() {
    // Bind the listener to the port.
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    println!("Listening");

    let db = Arc::new(Mutex::new(HashMap::new()));

    loop {
        // the second item contains the ip and port of the client
        let (socket, _) = listener.accept().await.unwrap();
        
        let db = db.clone();
        // a new task is spawned for each incoming socket
        tokio::spawn(async move {
            process(socket, db).await;
        });
    }
}


async fn process(socket: TcpStream, db: Db) {
    use mini_redis::Command::{self, Get, Set};

    // The `Connection` lets us read/write redis **frames** instead of
    // byte streams. The `Connection` type is defined by mini-redis.
    let mut connection = Connection::new(socket);

    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame).unwrap() {
            Set(cmd) => {
                let mut db = db.lock().unwrap();
                db.insert(cmd.key().to_string(), cmd.value().clone());
                Frame::Simple("OK".to_string())
            }
            Get(cmd) => {
                let db = db.lock().unwrap();
                if let Some(value) = db.get(cmd.key()) {
                    Frame::Bulk(value.clone())
                } else {
                    Frame::Null
                }
            }
            cmd => panic!("unimplemented {:?}", cmd),
        };

        connection.write_frame(&response).await.unwrap();
    }
}