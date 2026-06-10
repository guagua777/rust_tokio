use std::sync::{Mutex, MutexGuard};


// This works!
async fn increment_and_do_stuff(mutex: &Mutex<i32>) {
    {
        let mut lock: MutexGuard<i32> = mutex.lock().unwrap();
        *lock += 1;
    } // lock goes out of scope here

    do_something_async().await;
}

async fn do_something_async() {
    println!("Doing something async");
}


#[tokio::main]
async fn main() {
    let mutex = Mutex::new(0);
    increment_and_do_stuff(&mutex).await;
    println!("Count: {}", mutex.lock().unwrap());
}
