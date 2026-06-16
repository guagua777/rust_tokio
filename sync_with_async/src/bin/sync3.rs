use tokio::runtime::Builder;
use tokio::sync::mpsc;



// 这个文件演示了一个经典模式：从同步代码向异步运行时提交任务。逐块解释：

// 整体架构

// Plain Text

// 同步线程                    异步线程
// ┌──────────────┐          ┌─────────────────────────┐
// │ TaskSpawner  │          │  tokio Runtime           │
// │              │  channel │                          │
// │ spawn_task() ├─────────►│  recv.recv().await      │
// │ (同步方法)    │  Task    │       ↓                  │
// │              │          │  tokio::spawn(handle_task)│
// └──────────────┘          └─────────────────────────┘
// 核心问题：同步代码（没有 async 运行时）怎么调用异步函数？ 答案是用 channel 做桥梁。

// 逐段解析
// 1. 数据定义

// Rust

// pub struct Task {
//     name: String,
// }

// async fn handle_task(task: Task) {
//     println!("Got task {}", task.name);
// }
// Task 是要处理的任务，handle_task 是异步处理函数。问题是：谁调用 handle_task？必须是异步运行时。

// 2. TaskSpawner — 核心结构

// Rust

// #[derive(Clone)]
// pub struct TaskSpawner {
//     spawn: mpsc::Sender<Task>,  // 发送端：用来往 channel 里塞任务
// }
// 只持有 channel 的发送端，任何拿到 TaskSpawner 的同步代码都能发任务。

// 3. new() — 启动异步线程
// 这是最关键的部分，做了三件事：

// ① 创建 channel


// Rust

// let (send, mut recv) = mpsc::channel(16);
// 容量 16 的有界通道，send 给同步侧用，recv 给异步侧用。

// ② 创建单线程 tokio 运行时


// Rust

// let rt = Builder::new_current_thread()
//     .enable_all()
//     .build()
//     .unwrap();
// new_current_thread() = 单线程运行时（不是多线程），轻量级，适合这个场景。

// ③ 启动线程，运行异步事件循环


// Rust

// std::thread::spawn(move || {
//     rt.block_on(async move {
//         while let Some(task) = recv.recv().await {
//             tokio::spawn(handle_task(task));
//         }
//     });
// });
// 这里有三层嵌套，容易混淆：

// 层	代码	作用
// 外层	std::thread::spawn	开一个操作系统线程
// 中层	rt.block_on	在这个线程上启动 tokio 运行时
// 内层	while let Some(task) = recv.recv().await	异步循环，等待接收任务
// 循环逻辑：

// recv.recv().await 异步等待 channel 中有任务
// 收到任务后，tokio::spawn(handle_task(task)) 把任务丢给 tokio 运行时异步执行
// 所有 Sender 都被 drop 后，recv 返回 None，循环结束，线程退出
// 4. spawn_task() — 同步侧的入口

// Rust

// pub fn spawn_task(&self, task: Task) {
//     match self.spawn.blocking_send(task) {
//         Ok(()) => {},
//         Err(_) => panic!("The shared runtime has shut down."),
//     }
// }
// 关键点：blocking_send 而不是 send。

// 方法	上下文要求	行为
// send()	必须在 async 上下文中	非阻塞，返回 Future
// blocking_send()	可以在同步代码中调用	会阻塞当前线程直到 channel 有空间
// 这就是为什么同步代码能发任务——blocking_send 不需要 async 运行时。

// 使用示例
// 当前 main 是空的，可以这样用：


// Rust

// pub fn main() {
//     let spawner = TaskSpawner::new();  // 启动异步线程

//     spawner.spawn_task(Task { name: "task1".into() });  // 同步发任务
//     spawner.spawn_task(Task { name: "task2".into() });

//     std::thread::sleep(std::time::Duration::from_millis(100));
//     // spawner drop 后，Sender 被销毁，异步线程自动退出
// }
// 一句话总结
// TaskSpawner = 同步世界到异步世界的桥梁：同步代码通过 spawn_task() + blocking_send 把任务塞进 channel，异步线程里的 recv.recv().await 拿到任务后交给 tokio 运行时执行。






pub struct Task {
    name: String,
    // info that describes the task
}

async fn handle_task(task: Task) {
    println!("Got task {}", task.name);
}

#[derive(Clone)]
pub struct TaskSpawner {
    spawn: mpsc::Sender<Task>,
}

impl TaskSpawner {
    pub fn new() -> TaskSpawner {
        // Set up a channel for communicating.
        let (send, mut recv) = mpsc::channel(16);

        // Build the runtime for the new thread.
        //
        // The runtime is created before spawning the thread
        // to more cleanly forward errors if the `unwrap()`
        // panics.
        let rt = Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        std::thread::spawn(move || {
            rt.block_on(async move {
                while let Some(task) = recv.recv().await {
                    tokio::spawn(handle_task(task));
                }

                // Once all senders have gone out of scope,
                // the `.recv()` call returns None and it will
                // exit from the while loop and shut down the
                // thread.
            });
        });

        TaskSpawner {
            spawn: send,
        }
    }

    pub fn spawn_task(&self, task: Task) {
        match self.spawn.blocking_send(task) {
            Ok(()) => {},
            Err(_) => panic!("The shared runtime has shut down."),
        }
    }
}

pub fn main() {
    use std::thread;
    use std::time::Duration;

    let spawner = TaskSpawner::new();
    spawner.spawn_task(Task { name: "task1".to_string() });
    spawner.spawn_task(Task { name: "task2".to_string() });
    spawner.spawn_task(Task { name: "task3".to_string() });

    thread::sleep(Duration::from_millis(1000));
}