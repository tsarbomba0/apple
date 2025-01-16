use super::io_reactor::{Handle, IoReactor};
use mio::event::Event;
use mio::event::Source;
use std::marker::{Send, Sync};
use std::sync::Arc;

pub struct SchedIo<'i> {
    address: usize,
    handle: Arc<Handle>,
    io: Box<dyn Source + Send + Sync + 'i>,
}

impl<'i> SchedIo<'i> {
    pub fn new(
        addr: usize,
        handle: Arc<Handle>,
        io: Box<dyn Source + Send + Sync + 'i>,
    ) -> SchedIo<'i> {
        Self {
            address: addr,
            handle,
            io,
        }
    }

    pub fn dispatch(&self, ev: &Event) {}
}

unsafe impl Send for SchedIo<'_> {}
unsafe impl Sync for SchedIo<'_> {}
