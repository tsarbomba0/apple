#![feature(mpmc_channel)]

mod io;
mod runtime;
use crate::io::TcpStream;
use crate::io::{AsyncRead, AsyncWrite};
use mio::Interest;
use runtime::Runtime;

fn sleep_for_n_sec(n: u64) {
    std::thread::sleep(std::time::Duration::from_secs(n))
}

fn main() {
    Runtime::get();
    println!("Wersal!");
    Runtime::spawn(async_main());
    loop {
        std::thread::sleep(std::time::Duration::from_millis(500))
    }
}

async fn async_main() {
    let mut stream = TcpStream::new("127.0.0.1:8011", 0).expect("tcp socket fail");

    let rbuf = [5, 4, 3, 2, 1];
    let mut buf = [1u8; 5];

    Runtime::register(&mut stream, Interest::READABLE | Interest::WRITABLE).expect("register fail");
    let handle = Runtime::spawn(async move {
        let fut = stream.async_read(&mut buf);
        fut.await.expect("Failed reading!");
        println!("Buffer after read: {:#?}\n", buf.clone());

        let fut_w = stream.async_write(&rbuf);
        fut_w.await.expect("Failed writing!");
        println!("Writing done!");

        println!("\nLet's sleep for 5 seconds and test!");
        sleep_for_n_sec(5);
    });

    println!("Meow!");

    handle.await;

    println!("Meow 2!")

    // loop {
    //     println!("I am going to do this each second, haha!");
    //     sleep_for_n_sec(1)
    // }
}
