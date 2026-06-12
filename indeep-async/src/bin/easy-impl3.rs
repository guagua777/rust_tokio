use tokio::sync::Notify;
use std::time::{Duration, Instant};
use std::thread;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::sync::{Arc, Mutex};

use std::sync::mpsc;
use futures::task::{self, ArcWake};



// Notify::notified() 深度解析
// notified() 是 tokio::sync::Notify 的核心方法，它返回一个 Notified future，当收到通知时完成。结合你的 easy-impl3.rs 中的用法来理解：


// rust
// let notify = Arc::new(Notify::new());
// // ... 另一个线程中 ...
// notify_clone.notify_one();
// // 当前线程中
// notify.notified().await;
// 1. Notified 结构体

// rust
// pub struct Notified<'a> {
//     notify: &'a Notify,           // 关联的 Notify 实例
//     state: State,                 // 当前状态机状态
//     notify_waiters_calls: usize,  // 创建时 notify_waiters 的调用次数
//     waiter: Waiter,               // 等待者节点（链表节点）
// }

// enum State {
//     Init,     // 初始状态
//     Waiting,  // 已注册到等待队列
//     Done,     // 已完成（收到通知）
// }
// 2. 状态机流转（poll_notified 核心逻辑）
// Notified 实现了 Future<Output = ()>，其 poll 方法调用 poll_notified，核心是一个状态机循环：

// State::Init（第一次被 poll 时）
// 先检查 notify_waiters 是否被调用过：如果 notify_waiters_calls 计数不匹配，说明有广播通知，直接转到 Done
// 乐观尝试消费通知：用 CAS 尝试将 Notify 的内部状态从 NOTIFIED 改为 EMPTY
// ✅ 成功 → 消费掉了通知，转到 Done，返回 Poll::Ready(())
// ❌ 失败 → 没有待处理的通知，继续下一步
// 注册等待者：获取锁，将当前 waiter 插入 Notify 的等待链表，同时注册 waker，将状态转为 Waiting，返回 Poll::Pending
// State::Waiting（已注册等待，再次被 poll 时）
// 检查 waiter.notification 是否有值
// ✅ 有值 → 说明已被 notify_one 唤醒，转到 Done，返回 Poll::Ready(())
// ❌ 无值 → 更新 waker（因为 waker 可能变了），返回 Poll::Pending
// State::Done
// 返回 Poll::Ready(())，future 已完成。

// 3. 与 notify_one 的配合
// notify_one 的行为取决于 Notify 的内部状态：

// Notify 内部状态	notify_one 行为
// EMPTY	将状态设为 NOTIFIED（存一个"许可"）
// NOTIFIED	什么都不做（许可已存在，最多只存一个）
// WAITING	从等待链表中弹出一个 waiter，设置其 notification，唤醒对应的 waker
// 4. 在你的代码中的执行流程
// 看 easy-impl3.rs：


// rust
// async fn delay(dur: Duration) {
//     let notify = Arc::new(Notify::new());
//     let notify_clone = notify.clone();

//     thread::spawn(move || {
//         thread::sleep(when - now);  // 等待指定时间
//         notify_clone.notify_one();   // 时间到了，发通知
//     });

//     notify.notified().await;  // 等待通知
// }
// 执行过程：

// notify.notified().await 第一次被 poll（由 mini-tokio 驱动）
// Notified 进入 State::Init，此时 Notify 内部状态为 EMPTY（定时器线程还没睡够）
// CAS 尝试消费通知失败 → 注册 waiter 到等待链表 → 状态变为 Waiting → 返回 Poll::Pending
// mini-tokio 将该任务挂起
// 定时器线程睡够了，调用 notify_clone.notify_one()
// notify_one 发现状态是 WAITING，从链表弹出 waiter，设置 notification，调用 waker 唤醒任务
// waker 将任务重新放入 mini-tokio 的 channel
// mini-tokio 再次 poll 该任务 → Notified 处于 State::Waiting，检测到 notification 有值 → 转为 Done → 返回 Poll::Ready(())
// delay 函数返回，打印 "done"
// 5. 关键设计要点
// 许可机制（permit）：notify_one 可以在 notified().await 之前调用，通知会以"许可"形式存储（NOTIFIED 状态），后续的 notified().await 会立即完成
// FIFO 公平队列：多个等待者按先进先出顺序被唤醒
// 取消安全：如果 notified().await 被取消（drop），waiter 会从链表中移除，不会丢失通知
// notify_waiters_calls 计数：用于处理 notify_waiters()（广播）与 notified() 之间的竞态条件
// 本质上，notified() 就是一个异步信号量等待原语——它把"线程同步中的条件变量通知"模式变成了 async/await 风格，让你可以用 notify.notified().await 替代手写 poll + waker 的复杂逻辑。





async fn delay(dur: Duration) {
    println!("delay ...... ");
    let when = Instant::now() + dur;
    let notify = Arc::new(Notify::new());
    let notify_clone = notify.clone();

    thread::spawn(move || {
        let now = Instant::now();

        if now < when {
            thread::sleep(when - now);
        }

        notify_clone.notify_one();
    });


    notify.notified().await;
}



struct MiniTokio {
    // 收到后进行调度
    scheduled: mpsc::Receiver<Arc<Task>>,
    sender: mpsc::Sender<Arc<Task>>,
}


impl MiniTokio {
    fn run(&self) {
        while let Ok(task) = self.scheduled.recv() {
            task.poll();
        }
    }

    /// Initialize a new mini-tokio instance.
    fn new() -> MiniTokio {
        let (sender, scheduled) = mpsc::channel();

        MiniTokio { scheduled, sender }
    }

    /// Spawn a future onto the mini-tokio instance.
    ///
    /// The given future is wrapped with the `Task` harness and pushed into the
    /// `scheduled` queue. The future will be executed when `run` is called.
    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Task::spawn(future, &self.sender);
    }
}



/// A structure holding a future and the result of
/// the latest call to its `poll` method.
struct TaskFuture {
    future: Pin<Box<dyn Future<Output = ()> + Send>>,
    poll: Poll<()>,
}



impl TaskFuture {
    fn new(future: impl Future<Output = ()> + Send + 'static) -> TaskFuture {
        TaskFuture {
            future: Box::pin(future),
            // 初始状态
            poll: Poll::Pending,
        }
    }

    fn poll(&mut self, cx: &mut Context<'_>) {
        // Spurious wake-ups are allowed, even after a future has                                  
        // returned `Ready`. However, polling a future which has                                   
        // already returned `Ready` is *not* allowed. For this                                     
        // reason we need to check that the future is still pending                                
        // before we call it. Failure to do so can lead to a panic.
        if self.poll.is_pending() {
            // 再次调用poll
            self.poll = self.future.as_mut().poll(cx);
        }
    }
}


struct Task {
    // The `Mutex` is to make `Task` implement `Sync`. Only
    // one thread accesses `task_future` at any given time.
    // The `Mutex` is not required for correctness. Real Tokio
    // does not use a mutex here, but real Tokio has
    // more lines of code than can fit in a single tutorial
    // page.
    task_future: Mutex<TaskFuture>,
    // 为什么是sender，而不是receiver？
    executor: mpsc::Sender<Arc<Task>>,
}

impl Task {
    fn schedule(self: &Arc<Self>) {
        // 将此task发送到channel中，加入到队列里面，想想continuation
        self.executor.send(self.clone());
    }

    fn poll(self: Arc<Self>) {
        // Create a waker from the `Task` instance. This
        // uses the `ArcWake` impl from above.
        let waker = task::waker(self.clone());
        let mut cx = Context::from_waker(&waker);

        // No other thread ever tries to lock the task_future
        let mut task_future = self.task_future.try_lock().unwrap();

        // Poll the inner future
        task_future.poll(&mut cx);
    }

    // Spawns a new task with the given future.
    //
    // Initializes a new Task harness containing the given future and pushes it
    // onto `sender`. The receiver half of the channel will get the task and
    // execute it.
    fn spawn<F>(future: F, sender: &mpsc::Sender<Arc<Task>>)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task = Arc::new(Task {
            task_future: Mutex::new(TaskFuture::new(future)),
            executor: sender.clone(),
        });

        let _ = sender.send(task);
    }
}




impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.schedule();
    }
}

pub fn main() {
    let mut mini_tokio = MiniTokio::new();

    mini_tokio.spawn(async {      
        let future = delay(Duration::from_secs(10));

        println!("outer future ...... ");

        // await就是poll
        future.await;      

        println!("done");  
    });

    mini_tokio.run();
}