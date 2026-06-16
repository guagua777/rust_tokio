Tokio 官方文档 Graceful Shutdown（优雅关闭）完整解读
原文地址：https://tokio.rs/tokio/topics/shutdown
一、文档核心总纲
这篇文档专门解决Tokio 异步服务如何安全、优雅停机，避免直接杀进程导致：请求中断、数据库事务丢失、连接没释放、后台任务僵尸残留。
文档把优雅停机拆成固定三步骤，全文围绕这三步展开：
检测停机触发条件（什么时候该关）
广播停机信号给所有子模块 / 任务（通知全系统准备关闭）
等待所有任务收尾完成再退出（确保资源清理干净）
配套真实工程案例：mini-redis 服务源码，生产级可复用模板。
二、第一步：检测停机触发（什么时候要关闭）
最常见场景：用户按下 Ctrl+C、系统发送 SIGINT/SIGTERM（容器 / 云服务器重启）。
核心 API：tokio::signal::ctrl_c()
阻塞等待终端中断信号，收到后进入关闭流程
基础示例框架：
rust
运行
#[tokio::main]
async fn main() {
    // 启动服务后台任务
    let server_task = tokio::spawn(run_server());

    // 阻塞等待 Ctrl+C
    signal::ctrl_c().await.unwrap();
    println!("收到关闭信号，开始优雅停机");

    // 第二步、第三步逻辑写这里
}
补充：生产环境不止监听 Ctrl+C，还要捕获 SIGTERM（k8s/docker 停止信号），文档后面拓展了多信号监听写法。
三、第二步：通知全系统停机（统一广播关闭指令）
关键底层知识点：Tokio 是协作式取消
任务不会被强制中断，只有走到 .await 暂停点时，运行时才会检查是否收到取消信号；
如果一段代码无任何 .await（纯同步循环计算），哪怕发了关闭信号，任务也不会停下，会卡死停机流程。
官方推荐广播方案：oneshot /watch 通道（全局关闭标记）
文档主推 tokio::sync::watch::channel（广播通道）：
一个发送端，无数接收端；
发送关闭信号后，所有持有接收端的任务立刻感知；
轻量、可克隆，传给所有连接、工作协程。
标准流程：
主程序创建 watch 通道，初始状态「运行中」；
把 Receiver 克隆给每一个连接处理任务、后台定时任务；
收到 Ctrl+C 后，发送端发送「关闭」消息；
所有子任务用 select! 同时监听业务逻辑 + 关闭信号：
rust
运行
async fn handle_client(shutdown_rx: watch::Receiver<()>) {
    loop {
        select! {
            // 正常处理客户端消息
            msg = read_client() => msg,
            // 收到全局关闭信号，跳出循环收尾
            _ = shutdown_rx.changed() => {
                println!("客户端连接准备关闭");
                break;
            }
        }
    }
    // 释放连接、刷缓存、关闭IO资源
}
误区纠正（文档重点强调）
❌ 直接 abort() 粗暴杀死任务：未执行收尾逻辑，丢数据、连接泄漏；
❌ 仅丢弃 JoinHandle：任务不会停止，只是失去等待句柄，变成僵尸任务；
✅ 用广播通道主动通知，让任务自主收尾，属于优雅取消。
四、第三步：等待所有任务全部完成再退出
通知完系统停机后，不能直接 return，必须等待所有后台任务执行完清理逻辑。
两种等待方案
方案 1：收集所有 JoinHandle，逐个 await
适合任务数量可控（少量后台服务）：
rust
运行
let mut handles = Vec::new();
// 启动10个工作任务，收集句柄
for _ in 0..10 {
    let rx = shutdown_rx.clone();
    handles.push(tokio::spawn(worker(rx)));
}

// 收到关闭信号后，等待全部任务结束
for h in handles {
    h.await.unwrap();
}
方案 2：动态可变任务（服务海量客户端连接，mini-redis 用此方案）
客户端连接动态创建销毁，无法提前存固定 Vec，文档给出经典设计：
使用 tokio::sync::Semaphore / 计数器记录活跃任务；
每个客户端连接启动时计数器 + 1，退出前 - 1；
停机时循环等待计数器归零，确认无活跃连接。
Runtime 运行时本身的关闭行为（文档补充）
当 #[tokio::main] 主函数全部逻辑执行完毕、无剩余任务时，Runtime 自动销毁，但存在风险：
多线程 Runtime：未走到 .await 的任务会被直接丢弃，清理逻辑不执行；
手动 Runtime 提供三个关闭方法：
shutdown_background()：立刻释放主线程，后台任务放任跑完（不推荐，资源泄漏）；
shutdown_timeout(Duration)：给一段缓冲时间收尾，超时强制丢弃；
直接 drop (Runtime)：无限阻塞，直到所有任务结束（容易卡死）。
五、mini-redis 生产级完整架构（文档核心示例）
文档以 Redis 服务拆解分层停机设计，可直接套用在 Axum/Tonic 服务：
顶层入口 main：监听操作系统信号；
Server 层：持有全局 watch 关闭广播通道，停止接受新连接；
连接层：每个 TCP 连接持有广播接收器，收到信号后停止读取新请求，处理完当前请求再关闭 socket；
后台任务层：定时持久化、过期清理协程监听关闭信号，执行落盘、关闭文件；
等待汇总层：等待所有 TCP 连接、后台任务全部退出，再关闭 Runtime。
六、常见场景拓展
1. 同时监听 Ctrl+C + SIGTERM
rust
运行
let sigint = signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();
let sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
tokio::select! {
    _ = sigint.recv() => {},
    _ = sigterm.recv() => {},
}
2. 带超时保护的停机（防止任务卡死永远不退出）
收到关闭信号后，给 5s 让任务收尾，超时强制 abort 残留任务：
rust
运行
// 发关闭广播
shutdown_tx.send(()).unwrap();
// 给5秒等待
let wait_all = async {
    for h in handles { let _ = h.await; }
};
if tokio::time::timeout(Duration::from_secs(5), wait_all).await.is_err() {
    eprintln!("停机超时，强制终止残留任务");
    for h in handles { h.abort(); }
}
3. spawn_blocking 阻塞任务特殊处理
阻塞线程无法被协作取消，abort() 无效；
必须手动传递关闭标记，在阻塞循环内部主动判断退出条件。
七、文档核心思想总结
优雅停机 = 主动通知 + 自主收尾 + 等待全部完成，三者缺一不可；
禁止暴力 abort 或直接退出，优先用 watch 广播统一关闭信号；
所有长期运行任务（连接、定时、后台工作）必须嵌入 select! 监听关闭信号；
Tokio 取消是协作式，任务必须存在 .await 点才能响应关闭；
生产服务务必加停机超时，避免进程卡死无法退出；
区分「通知停止新业务」和「等待存量业务跑完」两个阶段，先拒新连接，再等存量处理完。
八、和之前 bridging 文档的关联
bridging：解决同步 ↔ 异步代码互相调用；
shutdown：解决异步程序安全退出、资源清理；
两者结合就是完整工程模板：同步主程序启动 Tokio Runtime + 监听系统信号 + 优雅停机释放 Runtime。