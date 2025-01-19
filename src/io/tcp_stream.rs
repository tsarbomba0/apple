use super::reactor::Direction;
use crate::Reactor;

use mio::event::Source;
use mio::net;
use mio::Token;
use std::future::Future;
use std::io::{self, Read};
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
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut buf1: [u8; 10] = [0u8; 10];
        let mut fut = self.as_mut();

        match fut.io.read(&mut buf1) {
            Ok(_size) => {
                println!("buf: {:#?}", buf1);
                Poll::Ready(())
            }

            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                println!("Would block, task gives a waker!");
                Reactor::attach_waker(cx, self.token, Direction::Read);

                Poll::Pending
            }

            Err(_e) => Poll::Ready(()),
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
    pub fn async_read<'a>(&'a mut self, buf: &'a mut [u8]) -> impl Future<Output = ()> + 'a {
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
