// 解释下这段代码，没看懂是什么意思，要表达的是什么
// 12:43
// 代码解析
// 这段代码演示的是 tokio::select! 在循环中的一个关键问题：如何避免 future 被重复取消。

// 核心问题：select! 会取消未胜出的分支
// 先看一个错误写法来理解问题：


// Rust

// // ❌ 错误示范
// loop {
//     tokio::select! {
//         _ = action() => break,       // 每次循环都创建新的 future
//         Some(v) = rx.recv() => { }
//     }
// }
// 每次循环迭代时，select! 只执行先就绪的那个分支，另一个分支的 future 会被 drop（取消）。下一次循环又重新创建 action()，之前已经完成的进度全部丢失。

// 如果 action() 是一个耗时操作（比如已经执行了 90%），但某次循环中 rx.recv() 先就绪了，action() 就被取消了，下次又从头开始 → 永远完不成。

// 正确写法：tokio::pin! + 引用

// Rust

// let operation = action();       // 创建一次 future
// tokio::pin!(operation);         // 将它固定在栈上

// loop {
//     tokio::select! {
//         _ = &mut operation => break,   // 传入引用，不消费 future
//         Some(v) = rx.recv() => {
//             if v % 2 == 0 {
//                 break;
//             }
//         }
//     }
// }
// 关键区别：

// 写法	每次循环	效果
// action()	创建新 future	之前的进度丢失 ❌
// &mut operation	传入同一 future 的引用	进度保留 ✅
// 逐行解释

// Rust

// let operation = action();   // 创建 future，但还没有开始执行
// tokio::pin!(operation);     // 固定在栈上，使其可以被 &mut 引用
//                             // 因为 select! 需要轮询(poll) future，
//                             // 而 poll 需要 Pin<&mut F>

// loop {
//     tokio::select! {
//         _ = &mut operation => break,
//         //   ^^^^^^^^^^^^^^  传入可变引用
//         //   如果这个分支没胜出，future 只是暂时不被轮询
//         //   下次循环继续从上次暂停的地方恢复

//         Some(v) = rx.recv() => {
//             if v % 2 == 0 {
//                 break;       // 收到偶数也退出循环
//             }
//         }
//     }
// }
// 执行流程示意

// Plain Text

// 循环第1次:  operation 进度 30% → rx.recv() 先就绪(v=1, 奇数) → operation 暂停
// 循环第2次:  operation 进度 60% → rx.recv() 先就绪(v=3, 奇数) → operation 暂停  
// 循环第3次:  operation 进度 90% → rx.recv() 先就绪(v=2, 偶数) → break 退出
//    或
// 循环第N次:  operation 完成 → break 退出
// 为什么要 pin!？
// select! 通过 &mut 引用来轮询 future，这要求 future 必须被 Pin 包裹，因为：

// Future 被轮询时可能包含自引用结构
// 如果 future 的内存地址变了，自引用会变成悬垂指针
// Pin 保证 future 不会被移动，地址固定
// tokio::pin!(operation) 宏做了这件事：把 operation 固定在栈上，之后只能通过 &mut 访问它。

// 一句话总结
// tokio::pin! + &mut 的模式，让同一个 future 在多次 select! 循环中保持进度，不会被重复创建和取消。




async fn action() {
    // Some asynchronous logic
}

#[tokio::main]
async fn main() {
    let (mut tx, mut rx) = tokio::sync::mpsc::channel::<i32>(128);    
    
    let operation = action();
    tokio::pin!(operation);
    
    loop {
        tokio::select! {
            _ = &mut operation => break,
            Some(v) = rx.recv() => {
                if v % 2 == 0 {
                    break;
                }
            }
        }
    }
}