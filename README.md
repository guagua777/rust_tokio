# rust_tokio
1. https://ryhl.io/blog/async-what-is-blocking/#the-rayon-crate
2. https://github.com/tokio-rs/mini-redis
3. https://github.com/tokio-rs/website
4. https://github.com/pretzelhammer/rust-blog/blob/master/posts/common-rust-lifetime-misconceptions.md#2-if-t-static-then-t-must-be-valid-for-the-entire-program
5. https://draft.ryhl.io/blog/shared-mutable-state/
6. https://draft.ryhl.io/blog/shared-mutable-state/

## 什么时候使用异步？
operations that cannot complete immediately are suspended to the background


## 从语义的角度如何理解
1. tokio运行时负责提交异步任务，即spawn，block_on等，其中异步任务即一个async（即continuation）
    - spawn的异步任务之间是并行的
    - 任务内部是串行的（即continuation里面是串行的）
2. await负责获取异步任务的结果

###
1. 只有continuation没有用，得把它放到调度器的队列里才可以，spawn和block_on等就是把continuation放到调度器的队列里
2. 放到队列里之后就成了并行的了


###
1. 任务内部是串行的
2. 任务之间是并行的

###
tokio::spawn 相当于 java里面的 Executor.execute();


###
Tasks are the unit of execution managed by the scheduler. 

Spawning the task submits it to the Tokio scheduler, which then ensures that the task executes when it has work to do. 

The spawned task may be executed on the same thread as where it was spawned, or it may execute on a different runtime thread. 

The task can also be moved between threads after being spawned.

### 
So far, when we wanted to add concurrency to the system, we spawned a new task

### send & sync
Types that can be sent to a different thread are Send

Types that can be concurrently accessed through immutable references are Sync.

### task
A task is an operation running on the Tokio runtime, created by the tokio::spawn or Runtime::block_on function. Tools for creating futures by combining them such as .await and join! do not create new tasks, and each combined part is said to be "in the same task".


### mini-tokio总结：
1. 没有channel的时候，遍历队列，取出任务，调用任务的poll方法
2. 使用channel和wake，
    - mini-tokio的spawn，包装Task的spawn
    - Task的spawn将future发送到channel中
    - mini-tokio的run方法，从channel中取出任务，调用任务的poll方法
    - 任务的poll方法，调用future的poll方法，如果future完成，返回future的结果，
    - 如果future没有完成，从context中获取waker，调用wake方法，再次将task发送到channel中
    - 然后run方法继续
3. 对比：
    - 没有channel的时候，遍历队列，取出任务，执行
    - 有channel的时候，
        1. 需要waker wake executor再次执行任务（此处为再次发送到channel中）
        2. 因为需要发送到channel里面，所以需要记录几个状态
            1. future的状态
            2. sender
    - outer future 如何调用 inner future，或者说是outer future如何包装inner future的？
        1. tokio内部把inner future包装在outer future中，参考main-future.rs，会自动生成一个枚举类，该枚举类实现Future trait，里面内容为outer future，在该outer future里面封装了inner future
    - 总归还是没有continuation的思路清晰

