use mio::event::Event;
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

    pub fn has_wakers(&self) -> bool {
        self.read_waker.is_some() || self.write_waker.is_some()
    }

    pub fn wake_with_event(&self, ev: &Event) {
        if let (true, Some(waker)) = (ev.is_readable(), &self.read_waker) {
            waker.wake_by_ref();
        }

        if let (true, Some(waker)) = (ev.is_writable(), &self.write_waker) {
            waker.wake_by_ref();
        }

        // todo: handle closing and stuff
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
