// main-future.rs


let r: &mut i32 = &mut x;  // r 内部是 &mut，但 r 本身是不可变绑定

*r = 42;                    // ✅ 可以，因为 *r 通过 &mut 写入


和 

 *
self
 = State1(future);


都是通过*来进行赋值，为什么r就只需要是
 &mut i32
，不需要是let mut r: &mut i32，而self就需要是mut 



关键区别：* 的实现路径不同
*r = 42（r: &mut i32）—— 语言原语
&mut i32 是原生可变引用，*r 是编译器内建的操作，不经过任何 trait 方法调用，直接解引用。语言规则直接允许你通过 &mut 写入，不需要 mut 绑定。

*self = State1(future)（self: Pin<&mut Self>）—— 经过 trait 方法
Pin 是一个库类型（包装类型），*self 实际上调用的是 DerefMut trait：


rust
// Pin 的 DerefMut 实现（简化）
impl<P: DerefMut> DerefMut for Pin<P> {
    fn deref_mut(&mut self) -> &mut P::Target {
        // ...
    }
}
注意签名：fn deref_mut(&mut self) —— 它需要 &mut Pin<&mut Self>，即对 Pin 本身的可变引用。

要创建 &mut self（对 self 的可变引用），self 必须是 mut 绑定。如果 self 不是 mut，你只能创建 &self（不可变引用），那就只能调用 Deref::deref(&self) 做只读访问，无法调用 DerefMut::deref_mut(&mut self) 做写入。

完整调用链对比

plainText
*r = 42  (r: &mut i32)
└── 语言内建：直接解引用可变引用，无需任何中间步骤
    ✅ 不需要 mut 绑定

*self = State1(future)  (self: Pin<&mut MainFuture>)
└── *self 调用 DerefMut::deref_mut
    ├── 需要 &mut self（对 Pin 的可变引用）
    │   └── 需要 self 是 mut 绑定
    │       └── 返回 &mut MainFuture
    │           └── 然后才能赋值
    ❌ 没有 mut 绑定 → 无法调用 deref_mut → 无法写入
更直观的类比
可以把 Pin 想象成一个保险箱：

类型	类比	* 做了什么
&mut i32	你直接拿着钥匙	* = 直接开门，一步到位
Pin<&mut T>	钥匙锁在保险箱里	* = 先开保险箱（需要 &mut Pin），再拿钥匙开门
mut self 就是开保险箱的权限。没有 mut，你只能透过保险箱的玻璃看（Deref::deref），但拿不到里面的钥匙（DerefMut::deref_mut）。

一句话总结
&mut T 的 * 是语言原语，直接操作；Pin<&mut T> 的 * 要经过 DerefMut::deref_mut(&mut self) 方法，而调用这个方法需要 &mut Pin，所以 self 必须是 mut 绑定。同样的 * 符号，背后的机制完全不同。