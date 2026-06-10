# rust_tokio

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
Tasks are the unit of execution managed by the scheduler. 

Spawning the task submits it to the Tokio scheduler, which then ensures that the task executes when it has work to do. 

The spawned task may be executed on the same thread as where it was spawned, or it may execute on a different runtime thread. 

The task can also be moved between threads after being spawned.

### send & sync
Types that can be sent to a different thread are Send

Types that can be concurrently accessed through immutable references are Sync.


### 
1. Update the MiniTokio struct.
