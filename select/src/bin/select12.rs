
pub fn get() -> Option<i32> {
    let a = None;
    let b = a?;
    println!("not print ....");
    Some(b)
}



pub fn main()  {
    let b = get();
    println!("{:?}", b);
}