#![feature(mpmc_channel)]

mod io;
mod runtime;
use crate::io::TcpStream;
use crate::io::{AsyncRead, AsyncWrite};
use mio::Interest;
use runtime::Runtime;
use std::sync::Arc;

fn sleep_for_n_sec(n: u64) {
    std::thread::sleep(std::time::Duration::from_secs(n))
}

fn main() {
    Runtime::build(4);
    Runtime::init(async_main());
}

async fn async_main() {
    let mut stream = TcpStream::new("127.0.0.1:8011", 0).expect("tcp socket fail");

    Runtime::register(&mut stream, Interest::READABLE | Interest::WRITABLE).expect("register fail");

    let arc_stream = Arc::new(stream);
    let stream1 = Arc::clone(&arc_stream);
    let stream2 = Arc::clone(&arc_stream);

    let handle1 = Runtime::spawn(async move {
        let rbuf = [5, 4, 3, 2, 1];
        let mut buf = [1u8; 5];

        let pipe = stream1.clone();
        let mut pipe_stream = pipe.as_ref();
        let fut = pipe_stream.async_read(&mut buf);
        fut.await.expect("Failed reading!");
        println!("Buffer after read: {:#?}\n", buf.clone());

        let fut_w = pipe_stream.async_write(&rbuf);
        fut_w.await.expect("Failed writing!");

        println!("\nLet's sleep for 1 second and test!");
        sleep_for_n_sec(1);
    });

    let handle2 = Runtime::spawn(async move {
        let rbuf = [5, 4, 3, 2, 1];
        let mut buf = [1u8; 5];

        let pipe = stream2.clone();
        let mut pipe_stream = pipe.as_ref();
        let fut = pipe_stream.async_read(&mut buf);
        fut.await.expect("Failed reading!");
        println!("Buffer after read: {:#?}\n", buf.clone());

        let fut_w = pipe_stream.async_write(&rbuf);
        fut_w.await.expect("Failed writing!");

        println!("\nLet's sleep for 1 second and test!");
        sleep_for_n_sec(1);
    });

    handle1.await;
    handle2.await;

    println!("Meow 2!")

    // loop {
    //     println!("I am going to do this each second, haha!");
    //     sleep_for_n_sec(1)
    // }
}
