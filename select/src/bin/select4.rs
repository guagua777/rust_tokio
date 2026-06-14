use tokio::net::TcpListener;
use tokio::sync::oneshot;
use std::io;
use tokio::net::TcpStream;


pub fn process(socket: TcpStream) {
    // 处理连接
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // 创建channel
    // impl<T> Future for Receiver<T> {
    let (tx, rx) = oneshot::channel();

    // 生成一个新的任务，发送消息到channel中
    tokio::spawn(async move {
        tx.send(()).unwrap();
    });

    // 创建监听器，监听端口3465
    let mut listener = TcpListener::bind("localhost:3465").await?;

    tokio::select! {
        _ = async {
            loop {
                let (socket, _) = listener.accept().await?;
                tokio::spawn(async move { process(socket) });
            }

            // Help the rust type inferencer out
            Ok::<_, io::Error>(())
        } => {}
        // 等待channel中的消息
        _ = rx => {
            println!("terminating accept loop");
        }
    }

    Ok(())
}