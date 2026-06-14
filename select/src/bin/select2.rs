use tokio::sync::oneshot;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};



// 为什么 &'static str 需要 'static
// 核心原因有两个层面：

// 1. 结构体不能持有未标注生命周期的引用
// 如果你写成 &str 而不是 &'static str：


// Rust

// // ❌ 编译失败
// struct MySelect {
//     rx1: oneshot::Receiver<&str>,   // 缺少生命周期
//     rx2: oneshot::Receiver<&str>,
// }
// 编译器会报错：missing lifetime specifier。你必须显式标注：


// Rust

// // ✅ 可以编译，但很麻烦
// struct MySelect<'a> {
//     rx1: oneshot::Receiver<&'a str>,
//     rx2: oneshot::Receiver<&'a str>,
// }

// impl<'a> Future for MySelect<'a> {
//     type Output = ();
//     fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
//         // ...
//     }
// }
// 生命周期参数 'a 会传染到所有使用 MySelect 的地方，代码变得复杂。

// 2. tokio::spawn 要求 'static
// 如果将来你想把这个 future spawn 到一个独立任务上：


// Rust

// tokio::spawn(MySelect { rx1, rx2 });
// tokio::spawn 的签名要求 future 满足 'static：


// Rust

// pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
// where
//     F: Future + Send + 'static,
// //                           ^^^^^^^
// 因为 spawn 出的任务可能在当前函数返回之后才执行，如果它持有对局部变量的引用，就会产生悬垂指针。'static 保证了引用在整个程序运行期间都有效。

// 总结
// 写法	能否编译	能否 spawn	复杂度
// &str	❌ 缺少生命周期	—	—
// &'a str	✅	❌ 不是 'static	高（生命周期传染）
// &'static str	✅	✅	低
// 所以 'static 既是为了满足 Rust 的语法要求（结构体中的引用必须有生命周期），也是为了实际使用中的灵活性（能被 spawn）。如果你不需要传引用，用 String 代替 &str 也可以完全避免这个问题。



struct MySelect {
    // 为什么要加'static
    rx1: oneshot::Receiver<&'static str>,
    rx2: oneshot::Receiver<&'static str>,
}

impl Future for MySelect {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if let Poll::Ready(val) = Pin::new(&mut self.rx1).poll(cx) {
            println!("rx1 completed first with {:?}", val);
            return Poll::Ready(());
        }

        if let Poll::Ready(val) = Pin::new(&mut self.rx2).poll(cx) {
            println!("rx2 completed first with {:?}", val);
            return Poll::Ready(());
        }

        Poll::Pending
    }
}

#[tokio::main]
async fn main() {
    let (tx1, rx1) = oneshot::channel();
    let (tx2, rx2) = oneshot::channel();

    // use tx1 and tx2

    MySelect {
        rx1,
        rx2,
    }.await;
}