use super::reactor::Direction;
use crate::Reactor;

use futures::Future;
use mio::Token;
use mio::event::Source;
use mio::net;
use std::io::{self, Read, Write};
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::task::{Context, Poll};

pub struct ReadFuture<'o> {
    io: &'o mut net::TcpStream,
    buf: &'o mut [u8],
    token: Token,
    //_pin: PhantomPinned,
}

impl<'o> Future for ReadFuture<'o> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<usize>> {
        let mut buf1: [u8; 10] = [0u8; 10];
        let mut fut = self.as_mut();

        match fut.io.read(&mut buf1) {
            Ok(size) => {
                println!("buf: {:#?}", buf1);
                Poll::Ready(Ok(size))
            }

            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                println!("Would block, task gives a waker!");
                Reactor::attach_waker(cx, self.token, Direction::Read);

                Poll::Pending
            }

            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

pub struct TcpStream {
    io: mio::net::TcpStream,
    token: Token,
    read: AtomicBool,
    write: AtomicBool,
}

impl TcpStream {
    pub fn new(addr: &str, tkn: usize) -> io::Result<TcpStream> {
        let address = match addr.parse() {
            Ok(o) => o,
            Err(_e) => panic!("Invalid address!"),
        };

        let tcp = net::TcpStream::connect(address)?;
        let token = Token(tkn);
        Ok(Self {
            io: tcp,
            token,
            read: AtomicBool::new(true),
            write: AtomicBool::new(true),
        })
    }

    /// ASYNC READ
    pub fn async_read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> impl Future<Output = io::Result<usize>> {
        ReadFuture {
            io: &mut self.io,
            buf,
            token: self.token.clone(),
            //_pin: PhantomPinned,
        }
    }
}

impl<'a> io::Read for &'a TcpStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut io = &self.io;
        io.read(buf)
    }
}

impl Source for TcpStream {
    fn register(
        &mut self,
        reg: &mio::Registry,
        token: Token,
        intr: mio::Interest,
    ) -> io::Result<()> {
        self.io.register(reg, token, intr)
    }

    fn reregister(
        &mut self,
        reg: &mio::Registry,
        token: Token,
        intr: mio::Interest,
    ) -> io::Result<()> {
        self.io.reregister(reg, token, intr)
    }

    fn deregister(&mut self, registry: &mio::Registry) -> io::Result<()> {
        self.io.deregister(registry)
    }
}
