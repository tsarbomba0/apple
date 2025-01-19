use mio::Token;
use std::task::Waker;

/// Represents a connection between a waker and the reactor
pub struct IoSource {
    read_waker: Option<Waker>,
    write_waker: Option<Waker>,
    token: Token,
}

impl IoSource {
    pub fn new(token: usize) -> IoSource {
        IoSource {
            read_waker: None,
            write_waker: None,
            token: Token(token),
        }
    }

    pub fn change_read_waker(&mut self, waker: &Waker) {
        self.read_waker = Some(waker.clone())
    }

    pub fn change_write_waker(&mut self, waker: &Waker) {
        self.write_waker = Some(waker.clone())
    }

    pub fn get_read_waker(&self) -> &Option<Waker> {
        &self.read_waker
    }

    pub fn get_write_waker(&self) -> &Option<Waker> {
        &self.write_waker
    }
}
