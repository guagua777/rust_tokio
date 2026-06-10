
// Rust transforms the async fn at compile time into a routine that operates asynchronously.
async fn say_world() {
    println!("world");
}

// 相当于这个
// fn say_world() -> Future<Output = ()> {
//     Future {
//         println!("world");
//     }
// }



#[tokio::main]
async fn main() {
    // Calling `say_world()` does not execute the body of `say_world()`.
    let op = say_world();

    // This println! comes first
    println!("hello");

    // Calling `.await` on `op` starts executing `say_world`.
    op.await;
}