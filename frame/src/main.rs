use bytes::Bytes;

mod connection;
mod connection2;
mod connection3;

enum Frame {
    Simple(String),
    Error(String),
    Integer(u64),
    Bulk(Bytes),
    Null,
    Array(Vec<Frame>),
}


// enum HttpFrame {
//     RequestHead {
//         method: Method,
//         uri: Uri,
//         version: Version,
//         headers: HeaderMap,
//     },
//     ResponseHead {
//         status: StatusCode,
//         version: Version,
//         headers: HeaderMap,
//     },
//     BodyChunk {
//         chunk: Bytes,
//     },
// }


use tokio::net::TcpStream;
use mini_redis::Result;

struct Connection {
    stream: TcpStream,
    // ... other fields here
}

impl Connection {
    /// Read a frame from the connection.
    /// 
    /// Returns `None` if EOF is reached
    pub async fn read_frame(&mut self)
        -> Result<Option<Frame>>
    {
        // implementation here
        Ok(None)
    }

    /// Write a frame to the connection.
    pub async fn write_frame(&mut self, frame: &Frame)
        -> Result<()>
    {
        // implementation here
        Ok(())
    }
}


fn main() {
    println!("Hello, world!");
}
