use tokio::net::{TcpListener, TcpStream};
use mini_redis::{Connection, Frame};

use bytes::Bytes;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type Db = Arc<Mutex<HashMap<String, Bytes>>>;


#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    println!("Listening");

    let db = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        // Clone the handle to the hash map.
        let db = db.clone();

        println!("Accepted");
        tokio::spawn(async move {
            process(socket, db).await;
        });
    }
}

// async fn process(socket: TcpStream) {
//     // The `Connection` lets us read/write redis **frames** instead of
//     // byte streams. The `Connection` type is defined by mini-redis.
//     let mut connection = Connection::new(socket);

//     if let Some(frame) = connection.read_frame().await.unwrap() {
//         println!("GOT: {:?}", frame);

//         // Respond with an error
//         let response = Frame::Error("unimplemented".to_string());
//         connection.write_frame(&response).await.unwrap();
//     }
// }

// async fn process(socket: TcpStream) {
//     use mini_redis::Command::{self, Get, Set};
//     use std::collections::HashMap;

//     // A hashmap is used to store data
//     let mut db = HashMap::new();

//     // Connection, provided by `mini-redis`, handles parsing frames from
//     // the socket
//     let mut connection = Connection::new(socket);

//     // Use `read_frame` to receive a command from the connection.
//     while let Some(frame) = connection.read_frame().await.unwrap() {
//         let response = match Command::from_frame(frame).unwrap() {
//             Set(cmd) => {
//                 println!("set: {:?}", cmd);
//                 // The value is stored as `Vec<u8>`
//                 db.insert(cmd.key().to_string(), cmd.value().to_vec());
//                 Frame::Simple("OK".to_string())
//             }
//             Get(cmd) => {
//                 println!("get: {:?}", cmd);
//                 if let Some(value) = db.get(cmd.key()) {
//                     // `Frame::Bulk` expects data to be of type `Bytes`. This
//                     // type will be covered later in the tutorial. For now,
//                     // `&Vec<u8>` is converted to `Bytes` using `into()`.
//                     Frame::Bulk(value.clone().into())
//                 } else {
//                     Frame::Null
//                 }
//             }
//             cmd => panic!("unimplemented {:?}", cmd),
//         };

//         // Write the response to the client
//         connection.write_frame(&response).await.unwrap();
//     }
// }

async fn process(socket: TcpStream, db: Db) {
    use mini_redis::Command::{self, Get, Set};

    // Connection, provided by `mini-redis`, handles parsing frames from
    // the socket
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

        // Write the response to the client
        connection.write_frame(&response).await.unwrap();
    }
}

