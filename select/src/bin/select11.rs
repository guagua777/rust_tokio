// 解释下这段代码的含义
// 15:15
// 代码解析
// 这段代码演示的是 tokio::select! 的前置条件（guard）机制，配合 Pin::set 实现动态替换 future。

// 关键语法：if !done

// Rust

// tokio::select! {
//     res = &mut operation, if !done => { ... }
// //                     ^^^^^^^^^
// //                     前置条件（guard）
//     Some(v) = rx.recv() => { ... }
// }
// if !done 是一个前置条件，它的作用是：

// 当 done == true 时，这个分支被禁用，select! 完全忽略它
// 当 done == false 时，这个分支正常参与 select! 竞争
// 完整执行流程
// 初始状态：operation = action(None)，done = false


// Plain Text

// 循环第1次:
//   ┌─ operation 分支: action(None) → 内部 input 是 None → 返回 None
//   │   guard: !done = !false = true ✅ 允许参与
//   │   → action(None) 立即返回 None
//   │   → done = true
//   │   → res 是 None，不打印，继续循环
//   │
//   └─ rx.recv() 分支: 等待接收

// 循环第2次:
//   ┌─ operation 分支: guard: !done = !true = false ❌ 被禁用！
//   │   → select! 完全跳过这个分支
//   │
//   └─ rx.recv() 分支: 收到 v=1 (奇数) → 什么都不做，继续循环

// 循环第3次:
//   ┌─ operation 分支: guard 仍然 false ❌ 被禁用
//   │
//   └─ rx.recv() 分支: 收到 v=3 (奇数) → 什么都不做，继续循环

// 循环第4次:
//   ┌─ operation 分支: guard 仍然 false ❌ 被禁用
//   │
//   └─ rx.recv() 分支: 收到 v=2 (偶数) → 
//       → operation.set(action(Some(2)))  // 替换为新的 future
//       → done = false                     // 重新启用 operation 分支

// 循环第5次:
//   ┌─ operation 分支: guard: !done = !false = true ✅ 重新启用
//   │   → action(Some(2)) 返回 Some("i = 2")
//   │   → 打印 "GOT = i = 2"，return 退出
//   │
//   └─ rx.recv() 分支: ...
// 三个关键机制
// 1. Guard（前置条件）


// Rust

// res = &mut operation, if !done => { ... }
// 没有 guard 时，action(None) 返回 None 后，下次循环 &mut operation 会立即再次就绪（返回 None），导致忙循环（busy loop）。guard 让我们能在 operation 完成后暂时禁用这个分支。

// 2. Pin::set — 替换 pinned future


// Rust

// operation.set(action(Some(v)));
// operation 被 pin! 固定在栈上，正常情况下不能移动或替换。但 Pin::set 是 Pin 提供的特殊方法，允许原地替换被固定的值。这样我们可以在同一个内存位置上放入新的 future。

// 3. done 标志的配合

// 状态	含义	operation 分支
// done = false	operation 还没完成或刚被重置	✅ 启用
// done = true	operation 已完成，等待被替换	❌ 禁用
// 一句话总结
// 这段代码实现了一个模式：一个长期运行的 operation 可以被外部消息动态替换。当 operation 完成后，用 guard 禁用它防止忙循环；当收到偶数时，用 Pin::set 替换为新的 operation，并重新启用该分支。




async fn action(input: Option<i32>) -> Option<String> {
    // If the input is `None`, return `None`.
    // This could also be written as `let i = input?;`
    let i = match input {
        Some(input) => input,
        None => return None,
    };
   
    // async logic here
    Some(format!("i = {}", i))
}

#[tokio::main]
async fn main() {
    let (mut tx, mut rx) = tokio::sync::mpsc::channel(128);
    
    let mut done = false;
    let operation = action(None);
    tokio::pin!(operation);
    
    tokio::spawn(async move {
        let _ = tx.send(1).await;
        let _ = tx.send(3).await;
        let _ = tx.send(2).await;
    });
    
    loop {
        tokio::select! {
            res = &mut operation, if !done => {
                done = true;

                if let Some(v) = res {
                    println!("GOT = {}", v);
                    return;
                }
            }
            Some(v) = rx.recv() => {
                if v % 2 == 0 {
                    // `.set` is a method on `Pin`.
                    operation.set(action(Some(v)));
                    done = false;
                }
            }
        }
    }
}