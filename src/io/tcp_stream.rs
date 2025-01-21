// crate imports
use super::reactor::Direction;
use crate::io::{AsyncRead, AsyncWrite};
use crate::Reactor;

// Mio imports
use mio::event::Source;
use mio::net;
use mio::Token;

/// std imports
use std::future::Future;
use std::io::{self, Read, Write};
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::task::{Context, Poll};

/// Future representing the operation of reading from a `TcpStream`.
pub struct ReadFuture<'o> {
    io: &'o net::TcpStream,
    buf: &'o mut [u8],
    token: Token,
}

impl Future for ReadFuture<'_> {
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let future = self.get_mut();

        match future.io.read(future.buf) {
            Ok(size) => Poll::Ready(Ok(size)),

            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                println!("Would block, attaching waker (Read)!");
                Reactor::attach_waker(cx, future.token, Direction::Read);

                Poll::Pending
            }

            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

/// Future representing the operation of writing to a `TcpStream`.
pub struct WriteFuture<'o> {
    io: &'o net::TcpStream,
    buf: &'o [u8],
    token: Token,
}

impl Future for WriteFuture<'_> {
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let pin_self = self.get_mut();
        match pin_self.io.write(pin_self.buf) {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                println!(
                    "Would block, attaching waker (Write) for Token: {}!",
                    pin_self.token.0
                );
                Reactor::attach_waker(cx, pin_self.token, Direction::Write);
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
            Ok(size) => Poll::Ready(Ok(size)),
        }
    }
}

/// TCP Socket connected to a listener.
pub struct TcpStream {
    io: mio::net::TcpStream,
    token: Token,
    read: AtomicBool,
    write: AtomicBool,
}

/// Impl TcpStream
impl TcpStream {
    /// Create a new TcpStream.
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
}

impl AsyncRead for TcpStream {
    /// Read x amount of bytes from this socket.
    /// It's asynchronous woo!!
    fn async_read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> impl Future<Output = io::Result<usize>> + 'a {
        ReadFuture {
            io: &mut self.io,
            buf,
            token: self.token,
        }
    }
}

impl AsyncRead for &TcpStream {
    /// Read x amount of bytes into `buf` from this socket asychronously.
    /// Returns a `Future` with `io::Result<usize>` as it's Output type.
    fn async_read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> impl Future<Output = io::Result<usize>> + 'a {
        ReadFuture {
            io: &self.io,
            buf,
            token: self.token,
        }
    }
}

impl AsyncWrite for TcpStream {
    /// Writes x amount of bytes from `buf` to this socket asychronously
    /// Returns a `Future` with `io::Result<usize>` as it's Output type.
    fn async_write<'a>(
        &'a mut self,
        buf: &'a [u8],
    ) -> impl Future<Output = io::Result<usize>> + 'a {
        WriteFuture {
            io: &self.io,
            buf,
            token: self.token,
        }
    }
}

impl AsyncWrite for &TcpStream {
    /// Writes x amount of bytes from `buf` to this asychronously
    /// Returns a `Future` with `io::Result<usize>` as it's Output type
    fn async_write<'a>(
        &'a mut self,
        buf: &'a [u8],
    ) -> impl Future<Output = io::Result<usize>> + 'a {
        WriteFuture {
            io: &self.io,
            buf,
            token: self.token,
        }
    }
}

impl io::Read for &TcpStream {
    /// Works by calling the underlying `io`'s `read` function.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut io = &self.io;
        io.read(buf)
    }
}

impl io::Write for &TcpStream {
    /// Works by calling the underlying `io`'s `write` function.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut io = &self.io;
        io.write(buf)
    }

    /// Works by calling the underlying `io`'s `flush` function.
    fn flush(&mut self) -> io::Result<()> {
        let mut io = &self.io;
        io.flush()
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
