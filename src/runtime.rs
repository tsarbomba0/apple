use crate::dummy_mutex::DummyMutex;
use crate::io::Handle;
use crate::Reactor;

use async_lock::OnceCell;
use futures::task::{self, ArcWake};

use mio::event::Source;
use mio::{Interest, Registry, Token};

use std::future::Future;
use std::pin::Pin;

use std::sync::mpmc::{self, Receiver, Sender};
use std::sync::Arc;
use std::task::{Context, Poll};

use std::io::Result as IoResult;
use std::thread;

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
    fn wake_by_ref(arc_self: &Arc<Self>) {
        println!("ArcWaking the task!");
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

            thread::spawn(move || loop {
                println!("Task loop!!!!");
                match receiver.recv() {
                    Ok(task) => task.poll(),
                    Err(e) => panic!("{}", e),
                }
            });

            runtime
        })
    }

    /// Spawns a task onto the Runtime.
    pub fn spawn<F>(task: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
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
