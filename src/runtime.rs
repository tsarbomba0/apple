use crate::Reactor;
use crate::dummy_mutex::DummyMutex;
use crate::io::Handle;

use async_lock::OnceCell;
use futures::task::{self, ArcWake};

use mio::event::{Event, Source};
use mio::{Interest, Registry, Token};

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;

use std::sync::Arc;
use std::sync::mpmc::{self, Receiver, Sender};
use std::task::{Context, Poll};

use std::io::Result as IoResult;
use std::thread;
use std::thread_local;

// Convenience type for the Futures used by the Runtime.
type RuntimeFuture<T> = Pin<Box<dyn Future + Send>>;

// Convenience function to make a new Task wrapped in an Arc<T>
fn make_arc_task<F>(future: F, sender: Sender<Arc<Task<T>>>) -> Arc<Task>
where
    F: Future + Send + 'static,
{
    Arc::new(Task::new(FutureTask::new(Box::pin(future)), sender))
}

struct FutureTask<T> {
    poll: Poll<T>,
    ft: RuntimeFuture<T>,
}

impl<T> FutureTask<T> {
    fn new(f: RuntimeFuture<T>) -> FutureTask<T> {
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

struct Task<T> {
    taskft: DummyMutex<FutureTask<T>>,
    exec: Sender<Arc<Task<T>>>,
}

impl<T> Task<T> {
    fn new(tsft: FutureTask<T>, exec: Sender<Arc<Task>>) -> Task<T> {
        let taskft = DummyMutex::new(tsft);
        Task { taskft, exec }
    }
    fn send(self: &Arc<Self>) -> Result<(), mpmc::SendError<Arc<Task<T>>>> {
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
    fn wake_by_ref(arc_self: &Arc<Self>) {
        match arc_self.send() {
            Ok(_) => {}
            Err(e) => panic!("{}", e),
        }
    }
}

/// Async runtime.
pub struct Runtime {
    /// Channels to send and receive tasks
    receiver: Receiver<Arc<Task>>,
    sender: Sender<Arc<Task>>,

    /// I/O Driver and handle.
    driver: Reactor,
    handle: Arc<Handle>,
}

impl Runtime {
    pub fn get() -> &'static Runtime {
        // Sender and Receiver channel for the Tasks
        let (sd, rc) = mpmc::channel();

        // Global variable to hold the Runtime.
        static RUNTIME: OnceCell<Runtime> = OnceCell::new();

        // Acquires the reference to the OnceCell<T> in the static variable
        // and initializes it with the Runtime
        RUNTIME.get_or_init_blocking(|| {
            // Driver and it's handle
            let driver = Reactor::new();

            // Runtime
            let runtime = Runtime {
                receiver: rc,
                sender: sd,
                driver: driver.0,
                handle: Arc::clone(&driver.1),
            };

            // Stuff to be borrowed into the threads
            let receiver = runtime.receiver.clone();

            thread::spawn(move || {
                loop {
                    println!("Task loop!!!!");
                    match receiver.recv() {
                        Ok(task) => task.poll(),
                        Err(e) => panic!("{}", e),
                    }
                }
            });

            runtime
        })
    }

    /// Spawns a task onto the Runtime.
    pub fn spawn<'a, F>(task: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        println!("Spawned a task!");
        let sender = Runtime::get().sender.clone();
        sender
            .send(make_arc_task(task, sender.clone()))
            .expect("failed to send task to runtime!")
    }

    // register device
    pub fn register(dev: &mut impl Source, token: Token, interest: Interest) -> IoResult<()> {
        let reg = Runtime::registry();

        reg.register(dev, token, interest)
    }

    /// Get registry.
    pub fn registry() -> &'static Registry {
        Runtime::get().handle.registry()
    }
}
