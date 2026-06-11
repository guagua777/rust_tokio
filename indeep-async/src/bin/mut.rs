
// 1. 一个是指向的位置可变，可以指向一个新的位置 let mut a = 
//      a本身是可变的
// 2. 一个是指向的内容可变，位置里面的内容可以被修改 let a: &mut 
//      a是一个引用，指向的内容是可变的

pub fn main() {

    let mut a = 10;
    let mut c = &a;
    // *c = 30;
    let f = 20;
    c = &f;
    // let mut b: &mut i32 = &a;
    // a的可变引用，
    let b: &mut i32 = &mut a;
    println!("{}", b);
    *b = 20;
    println!("{}", a);
    // b = &f;
}