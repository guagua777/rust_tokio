use std::sync::Mutex;

struct CanIncrement {
    mutex: Mutex<i32>,
}
impl CanIncrement {
    // This function is not marked async.
    fn increment(&self) {
        let mut lock = self.mutex.lock().unwrap();
        *lock += 1;
    }
}

async fn increment_and_do_stuff(can_incr: &CanIncrement) {
    can_incr.increment();
    do_something_async().await;
}

async fn do_something_async() {
    println!("Doing something async");
}

#[tokio::main]
async fn main() {
    let can_incr = CanIncrement {
        mutex: Mutex::new(0),
    };
    increment_and_do_stuff(&can_incr).await;
    println!("Count: {}", can_incr.mutex.lock().unwrap());
}