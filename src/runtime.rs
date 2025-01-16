use super::asyncio::io_reactor::IoReactor;
use crate::asyncio::io_reactor::Handle;
use crate::dummy_mutex::DummyMutex;
use async_lock::OnceCell;
use futures::task::{self, ArcWake};

use mio::event::{Event, Source};
use mio::{Interest, Registry, Token};

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;

use std::sync::mpmc::{self, Receiver, Sender};
use std::sync::Arc;
use std::task::{Context, Poll};

use std::io::Result as IoResult;
use std::thread;
use std::thread_local;

// Static variable for the Sender part of the channel
// used by the Runtime.
thread_local! {
    static SENDER: RefCell<Option<Sender<Arc<Task>>>> = RefCell::new(None);
}

// Convenience type for the Futures used by the Runtime.
type RuntimeFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

// Convenience function to make a new Task wrapped in an Arc<T>
fn make_arc_task<F>(future: F, sender: Sender<Arc<Task>>) -> Arc<Task>
where
    F: Future<Output = ()> + Send + 'static,
{
    Arc::new(Task::new(FutureTask::new(Box::pin(future)), sender))
}

struct FutureTask {
    poll: Poll<()>,
    ft: RuntimeFuture,
}

impl FutureTask {
    fn new(f: RuntimeFuture) -> FutureTask {
        FutureTask {
            poll: Poll::Pending,
            ft: f,
        }
    }
    fn poll(&mut self, cx: &mut Context<'_>) {
        if self.poll.is_pending() {
            self.poll = self.ft.as_mut().poll(cx)
        }
    }
}

struct Task {
    taskft: DummyMutex<FutureTask>,
    exec: Sender<Arc<Task>>,
}

impl Task {
    fn new(tsft: FutureTask, exec: Sender<Arc<Task>>) -> Task {
        let taskft = DummyMutex::new(tsft);
        Task { taskft, exec }
    }
    fn send(self: &Arc<Self>) -> Result<(), mpmc::SendError<Arc<Task>>> {
        self.exec.send(self.clone())
    }
    fn poll(self: Arc<Self>) {
        if self.taskft.poll.is_pending() {
            let waker = task::waker(self.clone());
            let mut cx = Context::from_waker(&waker);
            self.taskft.get_mut().poll(&mut cx)
        }
    }
}

impl ArcWake for Task {
    fn wake_by_ref(aself: &Arc<Self>) {
        aself.send();
    }
}

pub struct Runtime {
    /// Channels to send and receive tasks
    receiver: Receiver<Arc<Task>>,
    sender: Sender<Arc<Task>>,

    /// Sending events to threads
    poll_rcv: Receiver<Event>,
    poll_snd: Sender<Event>,

    /// I/O Driver and it's Handle
    driver: IoReactor<'static>,
    driver_handle: Arc<Handle>,
}

impl Runtime {
    pub fn get() -> &'static Runtime {
        // Sender and Receiver channel for the Tasks
        let (sd, rc) = mpmc::channel();

        // Sender and Receiver channel for Events
        let (poll_snd, poll_rcv) = mpmc::channel();

        // Driver and it's Reactor
        let driver = IoReactor::new();
        let driver_handle = driver.get_handle();

        // Thread local variable to hold the Runtime.
        thread_local! {
            static RUNTIME: OnceCell<Runtime> = OnceCell::new();
        }

        // Acquires the reference to the OnceCell<T> in the static variable
        // and initializes it with the Runtime
        RUNTIME.with(|runtime| {
            runtime.get_or_init_blocking(|| {
                // reactor
                let reactor = Runtime {
                    receiver: rc,
                    sender: sd,
                    poll_rcv,
                    poll_snd,
                    driver,
                    driver_handle,
                };

                // Stuff to be borrowed into the threads
                let receiver = reactor.receiver.clone();
                let sender = reactor.poll_snd.clone();

                thread::scope(|s| {
                    // Thread for polling tasks.
                    s.spawn(move || {
                        while let Ok(task) = receiver.recv() {
                            task.poll();
                        }
                    });
                    s.spawn(|| loop {
                        reactor.driver.poll_events();
                    })
                });

                reactor
            })
        })
    }

    /// Starts the I/O Reactor.
    fn start_io_reactor() -> Arc<Handle> {
        let (sender, receiver) = mpmc::channel();
        thread::spawn(move || {
            let io_reactor = IoReactor::new();
            match sender.send(io_reactor.get_handle()) {
                Ok(_) => println!("Gave handle!"),
                Err(e) => panic!("Oh god! {e:?}"),
            }
        });

        match receiver.recv() {
            Ok(handle) => handle,
            Err(e) => panic!("Oh god, no handle!: {e:?}"),
        }
    }

    /// Spawns a task onto the Runtime.
    pub fn spawn<F>(task: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        SENDER
            .with(|runtime| {
                let borrow = runtime.borrow();
                let unwrap = borrow.as_ref().unwrap();
                unwrap.send(make_arc_task(task, unwrap.clone()))
            })
            .unwrap();
    }

    // register device
    pub fn register(dev: &mut impl Source, token: Token, interest: Interest) -> IoResult<()> {
        let reg = Runtime::registry();

        reg.register(dev, token, interest)
    }

    /// Get registry.
    pub fn registry() -> &'static Registry {
        &*Runtime::get().driver_handle.get_registry()
    }

    // Get the channel for receiving events.
    pub fn get_event_chan() -> Receiver<Event> {
        Runtime::get().poll_rcv.clone()
    }
}
