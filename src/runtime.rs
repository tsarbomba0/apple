use crate::dummy_mutex::DummyMutex;
use crate::io::Handle;
use crate::task_handle::TaskHandle;
use crate::Reactor;

use async_lock::OnceCell;
use futures::task::{self, ArcWake};

use mio::event::Source;
use mio::{Interest, Registry, Token};

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use std::sync::mpmc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use std::io::Result as IoResult;
use std::thread;

// Convenience type for the Futures used by the Runtime.
type RuntimeFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Underlying task for the `Task` struct.
struct FutureTask {
    poll: Poll<()>,
    ft: RuntimeFuture,
}

impl FutureTask {
    /// Creates a new `FutureTask` with a type of `RuntimeFuture`
    /// `RuntimeFuture` is a type alias referring to `Pin<Box<dyn Future<Output = ()> + Send`
    fn new(f: RuntimeFuture) -> FutureTask {
        FutureTask {
            poll: Poll::Pending,
            ft: f,
        }
    }

    /// Polls the `Future` inside the `FutureTask`
    /// Only polls it if the poll returned earlier was `Poll::Pending`
    fn poll(&mut self, cx: &mut Context<'_>) {
        if self.poll.is_pending() {
            self.poll = self.ft.as_mut().poll(cx)
        }
    }
}

/// Task struct used by the Runtime
pub struct Task {
    taskft: DummyMutex<FutureTask>,
    exec: Sender<TaskRelated>,
    handle_waker: Mutex<Option<Waker>>,
}

impl Task {
    /// Creates a new `Task` from a `Sender<TaskRelated>` and a `FutureTask`
    fn new(tsft: FutureTask, exec: Sender<TaskRelated>) -> Task {
        let taskft = DummyMutex::new(tsft);
        Task {
            taskft,
            exec,
            handle_waker: Mutex::new(None),
        }
    }

    /// Convenience function to create a `Arc<Task>` from a type implementing `Future<Output = ()> + Send + 'static`
    fn arc_new<F>(future: F, sender: Sender<TaskRelated>) -> Arc<Task>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task = Task::new(FutureTask::new(Box::pin(future)), sender.clone());

        Arc::new(task)
    }

    /// Sends the `Task` to the Runtime
    fn send(self: &Arc<Self>) -> Result<(), mpmc::SendError<TaskRelated>> {
        self.exec.send(TaskRelated::Task(self.clone()))
    }

    /// Polls the internal `FutureTask`
    pub(crate) fn poll(self: Arc<Self>) {
        if self.taskft.poll.is_pending() {
            let waker = task::waker(self.clone());
            let mut cx = Context::from_waker(&waker);

            self.taskft.get_mut().poll(&mut cx)
        }
    }

    /// Convenience function to get the handle_waker field from a `Task`
    pub(crate) fn get_waker(&self) -> &Mutex<Option<Waker>> {
        &self.handle_waker
    }

    /// Checks if the Task is Pending or Ready
    pub(crate) fn ready(self: Arc<Task>) -> bool {
        !self.taskft.poll.is_pending()
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

/// Enum generalising the Task and TaskHandle types to let them be used without a `match` statement
pub enum TaskRelated {
    Task(Arc<Task>),
    Handle(Arc<TaskHandle>),
}

/// Async runtime.
pub struct Runtime {
    /// Channels to send and receive tasks
    receiver: Receiver<TaskRelated>,
    sender: Sender<TaskRelated>,

    /// I/O Driver and handle.
    driver: Reactor,
    handle: Arc<Handle>,

    /// Hashmap mapping `Arc<Task>` and their ids,
    map: HashMap<usize, Arc<Task>>,
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
                map: HashMap::with_capacity(1024),
            };

            // Stuff to be borrowed into the threads
            let receiver = runtime.receiver.clone();

            thread::spawn(move || loop {
                println!("Task loop!!!!");
                let task_related = match receiver.recv() {
                    Ok(task_related) => task_related,
                    Err(e) => panic!("{}", e),
                };

                match task_related {
                    TaskRelated::Task(task) => task.poll(),
                    TaskRelated::Handle(handle) => handle.handle_poll(),
                }
            });

            runtime
        })
    }

    /// Spawns a task onto the Runtime.
    pub fn spawn<F>(future: F) -> TaskHandle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let sender = Runtime::get().sender.clone();
        let task = Task::arc_new(future, sender.clone());
        let handle = TaskHandle::new(sender.clone());

        sender
            .send(TaskRelated::Task(task))
            .expect("failed to send task to runtime!");

        handle
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
