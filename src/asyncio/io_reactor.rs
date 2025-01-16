use super::schedio::SchedIo;
use mio::event::Source;
use mio::{Events, Interest, Poll, Registry, Token};
use slab::Slab;
use std::io::{Error, ErrorKind, Result};
use std::sync::{Arc, LockResult, Mutex, RwLock, RwLockReadGuard};

pub struct Handle {
    /// Poll
    pub(crate) poll: Mutex<Poll>,

    /// Registry
    reg: Registry,
}

impl Handle {
    /// Gets the Registry field from the Handle
    pub fn get_registry(&self) -> &Registry {
        &self.reg
    }

    /// Creates a new Handle
    fn new(reg: Registry, poll: Poll) -> Handle {
        Self {
            reg,
            poll: Mutex::new(poll),
        }
    }

    /// Gets the Poll field from the Handle
    pub fn get_poll(&self) -> &Mutex<Poll> {
        &self.poll
    }
}

type Sources<'o> = Arc<RwLock<Slab<SchedIo<'o>>>>;

pub struct IoReactor<'o> {
    /// Inner shared state
    handle: Arc<Handle>,

    /// Reused struct for events
    events: Mutex<Events>,

    /// I/O Sources
    srcs: Sources<'o>,
}

impl<'o> IoReactor<'o> {
    /// Constructs a new IoReactor.
    pub fn new() -> (IoReactor<'o>, Arc<Handle>) {
        let events = Events::with_capacity(1024);
        // Not being able to create a poll is fatal.
        let poll = Poll::new().expect("Failed to create poll!");
        // Not being able to obtain an owned registry is fatal.
        let reg = poll
            .registry()
            .try_clone()
            .expect("failed to get an owned registry");
        let srcs = Arc::new(RwLock::new(Slab::with_capacity(1024)));

        let handle = Handle::new(reg, poll);

        let io = IoReactor {
            handle: Arc::new(handle),
            events: Mutex::new(events),
            srcs,
        };

        let h = Arc::clone(&io.handle);

        (io, h)
    }

    /// Register device in the reactor's registry
    pub fn register<T>(&'o self, src: &'o mut T, intr: Interest) -> Result<()>
    where
        T: Source + Send + Sync,
    {
        let r = self
            .srcs_read()
            .expect("failed to get read lock on sources!");

        let token_num = r.vacant_key();
        self.handle.reg.register(src, Token(token_num), intr);
        // fix the types with WAKERS!
        let sched = SchedIo::new(token_num, self.get_handle(), src);

        let mut w = self.srcs.write().expect("failed to get write lock!");
        w.insert(sched);
        drop(w);

        Ok(())
    }

    /// Reregister device in the reactor's registry
    pub fn reregister(
        &'o self,
        src: &'o mut impl Source,
        tkn: Token,
        intr: Interest,
    ) -> Result<()> {
        let srcs = self.srcs_read().expect("couldn't get lock on sources!");
        if !srcs.contains(tkn.0) {
            let err = Error::new(
                ErrorKind::NotFound,
                "reregister operation with invalid token",
            );
            return Err(err);
        };

        self.handle.reg.reregister(src, tkn, intr)
    }

    /// Get handle to the I/O Reactor
    pub fn get_handle(&'o self) -> Arc<Handle> {
        Arc::clone(&self.handle)
    }

    /// Obtain a read lock on the RwLock holding the Slab.
    fn srcs_read(&'o self) -> LockResult<RwLockReadGuard<'o, Slab<SchedIo<'o>>>> {
        self.srcs.read()
    }

    /// Poll for events and dispatch them
    pub fn poll_events(&'o self) {
        let mut poll = self
            .handle
            .poll
            .lock()
            .expect("Failed to get lock on poll... somehow!");

        let mut events = self
            .events
            .lock()
            .expect("Failed to get the lock on the Mutex!");

        poll.poll(&mut events, None);

        let srcs = self.srcs_read().expect("couldn't get lock on sources!");
        for ev in events.iter() {
            println!("Ev! {:?}", ev);
            let regis = match srcs.get(ev.token().0) {
                None => {
                    panic!("Dispatching event to an unregistered device!");
                }
                Some(regis) => regis,
            };
            regis.dispatch(ev)
        }
    }
}
