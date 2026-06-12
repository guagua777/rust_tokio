use futures::future::poll_fn;
use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};
use std::task::{Context, Poll};
use std::thread;





// 问题出在 main 函数的执行流程上。让我逐步分析：

// 问题分析
// 看 esay-impl2.rs 中的关键代码：


// rust
// poll_fn(move |cx| {
//     let mut delay = delay.take().unwrap();
//     let res = Pin::new(&mut delay).poll(cx);
//     assert!(res.is_pending());
//     tokio::spawn(async move {
//         delay.await;
//     });

//     Poll::Ready(())  // ← 问题根源
// }).await;
// 执行流程如下：

// poll_fn 第一次被 poll：内部 Delay 被 poll 一次，因为 10ms 还没到，返回 Poll::Pending，同时启动了一个定时器线程（持有 poll_fn 上下文的 waker）。
// poll_fn 立即返回 Poll::Ready(())：这意味着 poll_fn 这个 future 立刻完成了。
// main 函数结束：因为 poll_fn.await 已经返回，main 随之返回。
// tokio 运行时关闭：运行时被 drop，所有通过 tokio::spawn 创建的尚未完成的任务被取消。
// tokio::spawn 里的 delay.await 还没来得及执行完，就被取消了，所以 "Hello world" 永远不会被打印。
// 根本原因
// tokio::spawn 是 "发射后不管"（fire-and-forget） 的——它不会阻塞当前函数。main 不会等待 spawned task 完成，运行时就关闭了。

// 修复方法
// 等待 spawned task 完成：


// rust
// poll_fn(move |cx| {
//     let mut delay = delay.take().unwrap();
//     let res = Pin::new(&mut delay).poll(cx);
//     assert!(res.is_pending());
//     let handle = tokio::spawn(async move {
//         delay.await;
//     });

//     cx.waker().wake_by_ref(); // 重新唤醒自己，等待 spawned task
//     if handle.is_finished() {
//         Poll::Ready(())
//     } else {
//         Poll::Pending
//     }
// }).await;
// 或者更简单地，直接 await handle：


// rust
// poll_fn(move |cx| {
//     let mut delay = delay.take().unwrap();
//     let res = Pin::new(&mut delay).poll(cx);
//     assert!(res.is_pending());
//     tokio::spawn(async move {
//         delay.await;
//     });
//     Poll::Ready(())
// }).await;

// // 在 poll_fn 外面等待一下，让 spawned task 有机会执行
// tokio::time::sleep(Duration::from_millis(100)).await;
// 最简洁的修复是直接 await spawned task 的 JoinHandle：


// rust
// let handle = tokio::spawn(async move {
//     delay.await;
// });
// handle.await.unwrap();
// 总结：poll_fn 立即返回 Ready，导致 main 瞬间结束，tokio 运行时关闭，spawned task 被取消，Delay 的 poll 方法再也没被调用，所以 "Hello world" 不会打印。

struct Delay {
    when: Instant,
}



impl Future for Delay {
    type Output = &'static str;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>)
        -> Poll<&'static str>
    {
        println!("inner future ...... ");
        if Instant::now() >= self.when {
            println!("Hello world");
            Poll::Ready("done")
        } else {
            // Get a handle to the waker for the current task
            let waker = cx.waker().clone();
            let when = self.when;

            // Spawn a timer thread.
            thread::spawn(move || {
                let now = Instant::now();

                if now < when {
                    thread::sleep(when - now);
                }

                waker.wake();
            });

            Poll::Pending
        }
    }
}


#[tokio::main]
async fn main() {
    let when = Instant::now() + Duration::from_millis(10);
    let mut delay = Some(Delay { when });

    // 无法直接调用poll，只能调用await
    // delay.take().poll();


    poll_fn(move |cx| {
        let mut delay = delay.take().unwrap();
        let res = Pin::new(&mut delay).poll(cx);
        assert!(res.is_pending());
        tokio::spawn(async move {
            delay.await;
        });

        Poll::Ready(())
    }).await;


    // thread::sleep(Duration::from_secs(100));
}


// cx 来自 poll_fn 的闭包参数。具体来说：

// rust
// poll_fn(move |cx| {   // ← cx 是 poll_fn 传给闭包的参数
//     ...
//     let res = Pin::new(&mut delay).poll(cx);
//     ...
// }).await;
// poll_fn 来自第 1 行的导入：


// rust
// use futures::future::poll_fn;
// 工作原理：

// poll_fn 的签名大致是：


// rust
// pub fn poll_fn<T, F>(f: F) -> PollFn<F>
// where
//     F: FnMut(&mut Context<'_>) -> Poll<T>,
// 它接受一个闭包，这个闭包的参数就是 &mut Context<'_>，也就是 cx。

// 当 poll_fn(...).await 被执行时，底层的 runtime 会：

// 创建一个 PollFn future
// 当这个 future 被 poll 时，runtime 会构造一个 Context（其中包含了当前任务的 Waker 等信息）
// 然后调用闭包，把这个 Context 作为参数传入——这就是 cx 的来源
// 所以 cx 本质上是 Tokio runtime 在 poll 这个 future 时自动提供的执行上下文，里面包含了当前任务的 Waker 等关键信息。你在第 134 行把它传给了 Pin::new(&mut delay).poll(cx)，这样 Delay 的 poll 实现就能拿到 Waker，在延时结束后通过 waker.wake() 唤醒当前任务。

