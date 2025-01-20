#![feature(mpmc_channel)]

mod dummy_mutex;
mod io;
use crate::io::TcpStream;
use io::Reactor;
use mio::Interest;
mod runtime;
use crate::io::{AsyncRead, AsyncWrite};
use runtime::Runtime;

fn main() {
    Runtime::get();
    println!("Wersal!");
    Runtime::spawn(async_main());
    loop {}
}

async fn async_main() {
    let mut stream = TcpStream::new("127.0.0.1:8011", 0).expect("tcp socket fail");

    let buf = [1u8; 5];
    let mut buf1 = [1u8; 5];

    Reactor::register(&mut stream, Interest::READABLE | Interest::WRITABLE).expect("register fail");
    let fut = stream.async_read(&mut buf1).await;
    let fut_w = stream.async_write(&buf).await;

    let mut stream1 = TcpStream::new("127.0.0.1:8011", 1).expect("tcp socket fail");

    Reactor::register(&mut stream1, Interest::READABLE | Interest::WRITABLE)
        .expect("register fail");
    let fut1 = stream1.async_read(&mut buf1).await;
    let fut1_w = stream1.async_write(&buf).await;

    let mut stream2 = TcpStream::new("127.0.0.1:8011", 2).expect("tcp socket fail");

    Reactor::register(&mut stream2, Interest::READABLE | Interest::WRITABLE)
        .expect("register fail");
    let fut2 = stream2.async_read(&mut buf1).await;
    let fut2_w = stream2.async_write(&buf).await;

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
}
