/// it is typically better to use one of the two other approaches.
/// 
/// 
use tokio::sync::Mutex; // note! This uses the Tokio mutex

// This compiles!
// (but restructuring the code would be better in this case)
async fn increment_and_do_stuff(mutex: &Mutex<i32>) {
    let mut lock = mutex.lock().await;
    *lock += 1;

    do_something_async().await;
} // lock goes out of scope here

async fn do_something_async() {
    println!("Doing something async");
}

#[tokio::main]
async fn main() {
    let mutex = Mutex::new(0);
    increment_and_do_stuff(&mutex).await;
    println!("Count: {}", mutex.lock().await);
}