这篇 Tokio 官方文档《Bridging with sync code》 核心讲解如何在以同步代码为主的 Rust 项目中混用 Tokio 异步代码（同步 ↔ 异步代码桥接），解决「同步上下文调用异步逻辑」的常见场景（比如 GUI 程序、传统同步服务嵌入异步网络逻辑），并给出三种主流实现方案、适用场景与代码示例Tokio。下面分模块通俗拆解全文内容：
一、文档核心背景
常规用法回顾
大家最常用 #[tokio::main] 把整个程序改成全异步，宏底层会自动创建 Tokio 运行时（Runtime），并用 block_on 阻塞主线程执行异步逻辑Tokio。
本文要解决的问题
很多项目不适合全异步（典型场景：GUI 程序、老旧同步业务、第三方同步库）：
主线程必须跑同步逻辑（如 GUI 界面）；
仅部分逻辑（网络请求、IO）需要用 Tokio 异步；
需求：在同步代码里调用异步函数，而非反过来。
二、前置知识点：#[tokio::main] 宏的本质
文档先拆解了异步入口宏的底层逻辑，这是理解桥接的基础：
原始异步代码
rust
运行
#[tokio::main] 
async fn main() {
    println!("Hello world");
}
宏展开后的真实代码（同步主函数 + 手动 Runtime）
rust
运行
fn main() {
    // 创建多线程 Tokio 运行时
    tokio::runtime::Builder::new_multi_thread()
        .enable_all() // 开启IO、定时器等驱动
        .build()
        .unwrap()
        // 阻塞当前线程，执行异步代码
        .block_on(async {
            println!("Hello world");
        })
}
关键结论：
block_on 是同步调用异步的核心 API—— 它会阻塞当前同步线程，直到传入的异步任务执行完毕。手动管理 Runtime + block_on，就是同步 / 异步桥接的基础方案Tokio。
三、方案一：Runtime + block_on（最常用：同步封装异步接口）
这是最简单、最基础的方案，文档用 mini-redis 异步客户端封装成同步客户端 做完整示例。
1. 核心思路
自定义一个同步包装结构体，内部持有：
原始的异步客户端（inner）；
独立的 Tokio Runtime 实例；
所有同步方法内部，统一调用 runtime.block_on(异步方法)，阻塞执行异步逻辑，对外暴露纯同步接口。
2. 关键 Runtime 选型：current_thread 单线程运行时
文档特意推荐 new_current_thread() 而非默认的多线程 new_multi_thread()：
current_thread：不创建额外线程，所有异步任务都在当前同步线程执行，轻量化、无线程开销；
适用：单次串行调用异步接口（如 Redis 单次 get/set），同一时间只跑一个异步任务；
特点：只有调用 block_on 时，异步任务才会运行；block_on 结束后，异步任务会暂停。
multi_thread：创建多线程工作池，后台任务可独立运行；
适用：需要后台持续跑异步任务的场景。
补充：enable_all() 必须加，否则 Runtime 无法使用 IO、定时器等核心功能。
3. 代码示例拆解（同步 Redis 客户端）
（1）包装结构体定义
rust
运行
pub struct BlockingClient {
    inner: 异步Redis客户端,
    rt: Tokio Runtime, // 专属运行时
}
（2）同步连接方法（构造函数）
同步的 connect 方法内部，用 rt.block_on 执行异步的连接逻辑。
（3）普通读写方法（get/set/set_expires/publish）
模板化写法：所有同步方法直接套一层 self.rt.block_on(异步方法)，一行代码完成桥接。
（4）特殊场景：订阅（subscribe）
Redis 订阅会把 Client 转为 Subscriber（状态变更），文档做了扩展：
BlockingClient::subscribe（同步方法）通过 block_on 执行异步订阅；
返回新的 BlockingSubscriber 同步结构体，继承原 Runtime；
订阅后的消息接收、退订等方法，依旧复用 block_on 桥接。
额外细节：如果异步方法本身就是非 async 同步方法（如 get_subscribed），直接调用即可，不需要 Runtime 和 block_on。
4. 适用场景
给异步库封装同步对外接口；
串行、单次调用异步逻辑（数据库、Redis、HTTP 单次请求）；
轻量场景，追求低开销。
四、方案二：Runtime + spawn（后台异步任务）
当你需要在同步主线程运行的同时，后台并发执行多个异步任务（而非串行等待），使用 runtime.spawn() 创建后台任务。
1. 核心思路
创建 multi_thread 多线程 Runtime（必须用多线程！）；
坑点：如果用 current_thread，后台 spawn 的任务永远不会执行（只有 block_on 才会唤醒）。
在同步主线程中，用 runtime.spawn() 批量创建异步后台任务；
主线程继续执行自己的同步逻辑（如 GUI 渲染）；
最后通过 block_on 等待所有后台任务结束。
2. 示例场景
GUI 程序：
主线程：跑 GUI 同步渲染逻辑；
Tokio 后台线程池：并发执行多个网络请求、IO 任务；
网络任务完成后，通过通道 / 共享变量通知 GUI 更新界面。
3. 任务结果通信方式（3 种）
spawn 返回 JoinHandle，除了用 block_on 等待结果，还可以：
mpsc 消息通道：异步任务通过通道向同步主线程推送数据；
共享变量 + 锁：适合 GUI 进度条这类高频状态更新；
Runtime 句柄（Handle）：克隆 Runtime 句柄，在代码任意位置继续创建后台任务。
4. 适用场景
主线程为同步逻辑（GUI、传统服务）；
需要并发后台异步任务（批量网络请求、定时任务）；
后台任务需要独立运行，不阻塞主线程。
五、方案三：独立线程 + 消息通道（解耦最强，灵活度最高）
这是最复杂但扩展性最强的方案，本质是异步 Runtime 完全跑在独立线程，同步主线程通过 mpsc 消息队列 和异步线程通信（典型的「Actor 模型」）。
1. 核心架构
新建一个独立线程，线程内部初始化 Tokio Runtime；
在线程内外创建 mpsc 双向消息通道：
同步主线程：发送任务指令（通过 blocking_send 同步发送）；
异步线程：循环监听通道，收到任务后用 tokio::spawn 执行异步逻辑；
线程完全隔离，同步、异步逻辑互不干扰。
2. 特点
解耦彻底：Runtime 生命周期独立，不受主线程影响；
可扩展：可增加信号量（Semaphore）限制并发任务数、增加返回通道接收异步结果；
样板代码多：需要手动管理线程、通道、生命周期。
3. 适用场景
大型项目、长期运行的后台异步服务；
同步和异步逻辑需要完全隔离；
复杂任务调度、双向通信（同步发指令，异步回结果）。
六、三大方案对比总结（快速选型）
表格
方案	核心 API	Runtime 类型	优点	缺点	最佳场景
方案一	block_on	current_thread	最简单、轻量、代码少	串行执行，阻塞主线程	异步库封装同步接口、单次串行 IO
方案二	spawn + block_on	multi_thread	后台并发，不阻塞主线程	任务依附当前 Runtime	GUI 后台请求、批量异步任务
方案三	独立线程 + mpsc	任选	解耦最强、灵活、稳定	代码量大、复杂度高	长期后台异步服务、复杂双向通信
七、全文核心思想提炼
核心桥梁：block_on 是同步调用异步的基础（阻塞同步线程，执行异步 Future）；
Runtime 选型原则：
串行单次调用 → current_thread（单线程，轻量）；
后台并发任务 → multi_thread（多线程池）；
场景优先：
只是简单封装接口 → 用方案一；
主线程要干活、后台跑异步任务 → 用方案二；
项目复杂、需要长期隔离异步服务 → 用方案三；
补充区分（文档顺带提及）：
本文：同步代码调用异步代码（用 block_on）；
反向（异步调用阻塞同步代码）：用 Tokio 另一个 API spawn_blocking（不在本文范围）。
简单来说：这篇文档就是教你不要强行把整个项目改成异步，而是根据业务形态，用三种不同姿势，让传统同步代码安稳地用上 Tokio 异步能力。