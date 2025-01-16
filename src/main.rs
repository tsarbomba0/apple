#![feature(mpmc_channel)]
mod asyncio;
use asyncio::async_tcp::AsyncTcpStream;
use asyncio::traits::AsyncRead;
mod delay_future;
mod dummy_mutex;
mod runtime;
mod tcp_stream;
use mio::Interest;
use runtime::Runtime;

fn main() {
    let runtime = Runtime::get();
    let reg = Runtime::registry();
    let mut tcp = AsyncTcpStream::new("127.0.0.1:8000", 3).unwrap();
    tcp.reg(reg, Interest::READABLE | Interest::WRITABLE);

    let future = tcp.read();
    Runtime::spawn(future);

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1))
    }
}
