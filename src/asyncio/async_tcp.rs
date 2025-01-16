use super::traits::AsyncRead;
use crate::Runtime;
use mio::event::{Event, Source};
use mio::net::TcpStream;
use mio::{Interest, Registry, Token};
use std::future::Future;
use std::io::{Read, Result, Write};
use std::pin::Pin;
use std::sync::mpmc::TryRecvError;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll as TaskPoll};
use std::thread;

struct AsyncReadFuture {
    data: Vec<u8>,
    done: bool,
    token: Arc<Token>,
    sock: AsyncTcpStream,
}

impl Future for AsyncReadFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> TaskPoll<()> {
        if !self.done {
            println!("AAAAAAAAAAAAAAAAAA");

            let waker = cx.waker();

            let tcp = self.sock.clone();

            let handle = thread::spawn(move || {
                let events = Runtime::get_event_chan();
                // loop to get stuff from a channel and process
                // haha!
                loop {
                    match events.try_recv() {
                        Ok(ev) => {
                            if ev.token() == tcp.token {
                                tcp.read_ready(&ev);
                                tcp.write_ready(&ev);
                            }
                        }
                        Err(e) if e == TryRecvError::Empty => break,
                        Err(_e) => println!("Error during reading channel in the future!"),
                    }
                }
            });
            handle.join();
            self.done = true;
            waker.wake_by_ref();
            TaskPoll::Pending
        } else {
            // probably shit
            //TaskPoll::Ready((*self.data).to_vec())
            TaskPoll::Ready(())
        }
    }
}

impl AsyncReadFuture {
    fn new(token: Arc<Token>, sock: AsyncTcpStream) -> AsyncReadFuture {
        Self {
            data: vec![],
            done: false,
            token,
            sock,
        }
    }
}

#[derive(Clone)]
pub struct AsyncTcpStream {
    sock: Arc<Mutex<TcpStream>>,
    token: Token,
}

impl AsyncTcpStream {
    pub fn new(addr: &str, num: usize) -> Result<AsyncTcpStream> {
        let sock_addr = addr.parse().expect("Invalid address!");
        let sock = TcpStream::connect(sock_addr)?;
        let token = Token(num);
        Ok(AsyncTcpStream {
            sock: Arc::new(Mutex::new(sock)),
            token,
        })
    }

    pub fn reg(&mut self, reg: &Registry, int: Interest) -> Result<()> {
        reg.register(self, self.token, int)
    }

    fn read_ready(&self, ev: &Event) {
        if ev.is_readable() {
            let mut tcp_sock = self.sock.lock().expect("Failed lock!");
            let mut buf = vec![];
            tcp_sock.read(&mut buf).expect("Failed read!");
            println!("{:#?}", buf);
        };
    }
    fn write_ready(&self, ev: &Event) {
        if ev.is_writable() {
            let mut tcp_sock = self.sock.lock().expect("Failed lock at write!");

            let buf = vec![8u8, 8];
            tcp_sock.write(&buf).expect("Failed write!");
        }
    }
    fn get_lock(&self) -> MutexGuard<TcpStream> {
        self.sock.lock().expect("Failed to lock tcp stream!")
    }
}

impl Source for AsyncTcpStream {
    fn register(&mut self, reg: &Registry, token: Token, interest: Interest) -> Result<()> {
        let mut sock = self.get_lock();
        sock.register(reg, token, interest)
    }
    fn deregister(&mut self, registry: &Registry) -> Result<()> {
        let mut sock = self.get_lock();
        sock.deregister(registry)
    }

    fn reregister(&mut self, reg: &Registry, token: Token, interest: Interest) -> Result<()> {
        let mut sock = self.get_lock();
        sock.reregister(reg, token, interest)
    }
}

impl AsyncRead for AsyncTcpStream {
    fn read(&self) -> impl Future<Output = ()> {
        AsyncReadFuture::new(Arc::new(self.token), self.clone())
    }
}
