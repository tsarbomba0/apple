#![feature(mpmc_channel)]

mod dummy_mutex;
mod io;
use crate::io::TcpStream;
use io::Reactor;
use mio::Interest;
mod runtime;
use futures::Future;
use runtime::Runtime;
use std::pin::pin;

fn main() {
    Runtime::get();
    println!("Wersal!");
    let mut stream = TcpStream::new("127.0.0.1:8011", 1).expect("tcp socket fail");

    Reactor::register(&mut stream, Interest::READABLE | Interest::WRITABLE).expect("register fail");
    let mut buf = vec![];
    let fut = stream.async_read(&mut buf);

    // let fut1 = async move {
    //     let mut buf = vec![];
    //     let a = stream.async_read(&mut buf).await;
    //     println!("{:?}", a);
    // };
    // let fut2 = async move {
    //     let mut buf = vec![];
    //     let a = stream.async_read(&mut buf).await;
    //     println!("{:?}", a);
    // };
    Runtime::spawn(fut);

    loop {}
}
