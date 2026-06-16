这个文件实现了一个 异步 TCP 回显服务器（Echo Server 的变体），并附带了单元测试。下面逐段解释：

1. main — 启动 TCP 服务器

Rust

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    loop {
        let Ok((mut socket, _)) = listener.accept().await else {
            eprintln!("Failed to accept client");
            continue;
        };

        tokio::spawn(async move {
            let (reader, writer) = socket.split();
            handle_connection(reader, writer)
                .await
                .expect("Failed to handle connection");
        });
    }
}
TcpListener::bind("127.0.0.1:8080") 在本地 8080 端口监听 TCP 连接。
loop 不断接受新客户端连接。
listener.accept().await 返回 (socket, address)，这里用 _ 忽略了客户端地址。
socket.split() 把一个 TcpStream 拆分成读和写两半：
reader：只能读
writer：只能写
这样可以分别传给处理函数，读写互不干扰。
tokio::spawn 为每个客户端连接启动一个独立的异步任务，这样多个客户端可以并发处理。
2. handle_connection — 处理单个客户端连接

Rust

async fn handle_connection<Reader, Writer>(
    reader: Reader,
    mut writer: Writer,
) -> std::io::Result<()>
where
    Reader: AsyncRead + Unpin,
    Writer: AsyncWrite + Unpin,
使用泛型而不是具体的 TcpStream，这是关键设计！
AsyncRead / AsyncWrite 是 Tokio 的异步读写 trait，任何实现了这两个 trait 的类型都能用。
这样在测试时可以传入模拟的 reader/writer，不必真的开 TCP 连接。
Unpin 是因为异步读写需要 self 是 Unpin 的（mut 引用可以安全移动）。

Rust

    let mut line = String::new();
    let mut reader = BufReader::new(reader);

    loop {
        if let Ok(bytes_read) = reader.read_line(&mut line).await {
            if bytes_read == 0 {
                break Ok(());
            }
            writer
                .write_all(format!("Thanks for your message.\r\n").as_bytes())
                .await
                .unwrap();
        }
        line.clear();
    }
BufReader::new(reader) 包装 reader，提供 read_line 等按行读取的便捷方法。
read_line(&mut line) 读取一行（到 \n 为止），追加到 line 中。
bytes_read == 0 表示客户端关闭了连接（EOF），退出循环。
每读到一行，就回复 "Thanks for your message.\r\n"。
line.clear() 每轮清空，准备读下一行。
3. client_handler_replies_politely — 单元测试

Rust

#[tokio::test]
async fn client_handler_replies_politely() {
    let reader = tokio_test::io::Builder::new()
        .read(b"Hi there\r\n")
        .read(b"How are you doing?\r\n")
        .build();
    let writer = tokio_test::io::Builder::new()
        .write(b"Thanks for your message.\r\n")
        .write(b"Thanks for your message.\r\n")
        .build();
    let _ = handle_connection(reader, writer).await;
}
这就是泛型设计的威力所在：

tokio_test::io::Builder 可以构建模拟的 I/O 对象，不需要真正的网络连接。
模拟 reader：预设客户端会发送两行数据：
"Hi there\r\n"
"How are you doing?\r\n"
模拟 writer：预设期望服务器会写入两次：
"Thanks for your message.\r\n"
"Thanks for your message.\r\n"
如果 handle_connection 写入的内容和预设不匹配，测试会失败。
整体数据流

Plain Text

客户端连接 → TcpListener.accept()
    ↓
socket.split() → (reader, writer)
    ↓
handle_connection(reader, writer)
    ↓
循环: 读一行 → 回复 "Thanks for your message."
    ↓
客户端断开 → bytes_read == 0 → 退出
核心设计要点
要点	说明
socket.split()	读写分离，允许独立操作
泛型 AsyncRead/AsyncWrite	解耦具体 I/O 实现，方便测试
tokio_test::io::Builder	模拟 I/O，无需真实网络，测试快速可靠
BufReader	提供按行读取能力
tokio::spawn	每个连接一个任务，支持并发