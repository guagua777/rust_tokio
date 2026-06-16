use async_stream::stream;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio_stream::StreamExt;

struct Delay {
    when: Instant,
}

impl Future for Delay {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.when {
            Poll::Ready(())
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}





// 解释下yield
// 11:53
// yield 的解释
// yield 在 stream! 宏中的作用是：向流中产出一个值，然后暂停执行，等待消费者下次拉取时再继续。

// 对比理解
// 你可以把 stream! 里的代码想象成一个"可以暂停的函数"：

// 关键字	行为
// return	返回值，函数结束，再也不会继续
// yield	返回值，暂停在这里，下次调用时从暂停处继续
// 结合你的代码看

// Rust

// stream! {
//     let mut when = Instant::now();
//     for _ in 0..3 {
//         let delay = Delay { when };
//         delay.await;     // ① 等待延迟完成
//         yield ();         // ② 产出 ()，暂停！等消费者下次 .next()
//         when += Duration::from_millis(10);  // ③ 消费者下次拉取时，从这里继续
//     }
// }
// 执行流程：


// Plain Text

// 第1次 next() → delay.await 等待 → yield () → 返回 Some(()) → 暂停
// 第2次 next() → when += 10ms → delay.await 等待 → yield () → 返回 Some(()) → 暂停
// 第3次 next() → when += 10ms → delay.await 等待 → yield () → 返回 Some(()) → 暂停
// 第4次 next() → when += 10ms → for 循环结束 → 返回 None → 流结束
// 等价于 stream2.rs 中的什么？
// 在 stream2.rs 中，yield 的效果等价于 Stream::poll_next 返回 Poll::Ready(Some(value))：


// Rust

// // stream2.rs 手动实现 Stream
// impl Stream for Interval {
//     fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<()>> {
//         match Pin::new(&mut self.delay).poll(cx) {
//             Poll::Ready(_) => {
//                 // ...
//                 Poll::Ready(Some(()))  // ← 这就等价于 yield ()
//             }
//             Poll::Pending => Poll::Pending,
//         }
//     }
// }
// 一句话总结
// yield = "我产出这个值，但我还没完，下次再来找我的时候我继续往下走"。它是 stream! 宏的核心语法，让你用类似写普通函数的方式定义流，而不需要手动实现 Stream trait 的状态机。



#[tokio::main]
async fn main() {
    let s = stream! {
        let mut when = Instant::now();
        for _ in 0..3 {
            let delay = Delay { when };
            delay.await;
            yield ();
            when += Duration::from_millis(10);
        }
    };

    tokio::pin!(s);

    while let Some(_) = s.next().await {
        println!("tick");
    }
}