use tokio::sync::Notify;
use std::time::{Duration, Instant};
use std::thread;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::sync::{Arc, Mutex};

use std::sync::mpsc;
use futures::task::{self, ArcWake};



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


#[tokio::main]
pub async fn main() {

    delay(Duration::from_secs(10)).await;
    println!("done");

}