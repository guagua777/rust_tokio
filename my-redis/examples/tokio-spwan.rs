#[tokio::main]
async fn main() {


    // A Tokio task is an asynchronous green thread. 
    // They are created by passing an async block to tokio::spawn. 

    // spawn 放入调度器
     let handle = tokio::spawn(async {
            println!("spawn start");
            // Do some async work
            // 放入调度器，同时获取任务的结果
            return_string().await
            // return_string()
        });

        // Do some main work
        println!("main work。。。。。。");

        // 为什么handle也能await
        // 为JoinHandle重写了Future
        // await获取任务的结果
        let out = handle.await.unwrap();
        // println!("GOT {}", out.await);
        println!("GOT {}", out);

    // loop {
    //     let handle = tokio::spawn(async {
    //         // Do some async work
    //         "return value"
    //     });

    //     // Do some other work

    //     let out = handle.await.unwrap();
    //     println!("GOT {}", out);
    // }
    
}


async fn return_string() -> String {
    "i am a string".into()
}