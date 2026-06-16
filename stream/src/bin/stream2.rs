use std::pin::Pin;
use std::task::{Context, Poll};
use tokio_stream::Stream;
use std::time::{Duration, Instant};
use tokio_stream::StreamExt;

// pub trait Stream {
//     type Item;

//     fn poll_next(
//         self: Pin<&mut Self>, 
//         cx: &mut Context<'_>
//     ) -> Poll<Option<Self::Item>>;

//     fn size_hint(&self) -> (usize, Option<usize>) {
//         (0, None)
//     }
// }

struct Delay {
    when: Instant,
}

impl Future for Delay {
    type Output = &'static str;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>)
        -> Poll<&'static str>
    {
        if Instant::now() >= self.when {
            println!("Hello world");
            Poll::Ready("done")
        } else {
            // Ignore this line for now.
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

struct Interval {
    rem: usize,
    delay: Delay,
}

impl Interval {
    fn new() -> Self {
        Self {
            rem: 3,
            delay: Delay { when: Instant::now() }
        }
    }
}

impl Stream for Interval {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>)
        -> Poll<Option<()>>
    {
        if self.rem == 0 {
            // No more delays
            return Poll::Ready(None);
        }

        match Pin::new(&mut self.delay).poll(cx) {
            Poll::Ready(_) => {
                let when = self.delay.when + Duration::from_millis(10);
                self.delay = Delay { when };
                self.rem -= 1;
                Poll::Ready(Some(()))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[tokio::main]
async fn main() {
    let mut interval = Interval::new();

    tokio::pin!(interval);


    while let Some(_) = interval.next().await {
        println!("tick");
    }
}