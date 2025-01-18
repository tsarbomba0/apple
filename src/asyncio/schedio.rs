use super::io_reactor::Handle;
use std::marker::{Send, Sync};
use std::sync::Arc;
use std::task::Waker;

/// Relates a Task's Waker and the Handle of the Reactor
pub struct SchedIo {
    address: usize,
    handle: Arc<Handle>,
    waker: Option<Waker>,
}

// impl SchedIo
impl SchedIo {
    pub fn new(address: usize, handle: Arc<Handle>) -> SchedIo {
        Self {
            address,
            handle,
            waker: None,
        }
    }

    /// Sets a waker for this IO.
    pub fn set_waker(&mut self, waker: &Waker) {
        self.waker = Some(waker.clone_from(&waker));
    }

    /// Wakes up the IO.
    pub fn wake(&self) {
        // Will panic if there is no waker.
        self.waker.unwrap().wake()
    }
}

unsafe impl Send for SchedIo {}
unsafe impl Sync for SchedIo {}
