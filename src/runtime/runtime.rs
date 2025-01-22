use crate::io::Handle;
use crate::io::Reactor;
use crate::runtime::MutCell;
use crate::runtime::TaskHandle;

use async_lock::OnceCell;
use futures::task::{self, ArcWake};

use mio::event::Source;
use mio::{Interest, Registry};

use std::future::Future;
use std::pin::Pin;

use std::sync::mpmc::{self, Receiver, Sender};
use std::sync::Arc;
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
    taskft: MutCell<FutureTask>,
    exec: Sender<Arc<Task>>,
    waker: MutCell<Waker>,
}

impl std::ops::Drop for Task {
    fn drop(&mut self) {
        println!("Dropped a task!");
        self.waker.get().wake_by_ref();
    }
}

impl Task {
    /// Creates a new `Task` from a `Sender<TaskRelated>` and a `FutureTask`
    fn new(tsft: FutureTask, exec: Sender<Arc<Task>>) -> Task {
        let taskft = unsafe { MutCell::new(tsft) };
        Task {
            taskft,
            exec,
            waker: unsafe { MutCell::new(Waker::noop().clone()) },
        }
    }

    /// Convenience function to create a `Arc<Task>` from a type implementing `Future<Output = ()> + Send + 'static`
    fn arc_new<F>(future: F, sender: Sender<Arc<Task>>) -> Arc<Task>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task = Task::new(FutureTask::new(Box::pin(future)), sender.clone());

        Arc::new(task)
    }

    /// Sends the `Task` to the Runtime
    fn send(self: &Arc<Self>) -> Result<(), mpmc::SendError<Arc<Task>>> {
        self.exec.send(self.clone())
    }

    pub(crate) fn change_waker(self: &Arc<Self>, waker: &Waker) {
        if !self.waker.will_wake(waker) {
            // Safety: Only one thread will access this.
            unsafe { *self.waker.get_mut() = waker.clone() }
        }
    }

    /// Polls the internal `FutureTask`
    pub(crate) fn poll(self: Arc<Self>) {
        if self.taskft.poll.is_pending() {
            let waker = task::waker(self.clone());
            let mut cx = Context::from_waker(&waker);

            // Safety:
            // This will let only 1 thread access this mutably.
            unsafe { self.taskft.get_mut().poll(&mut cx) };
        }
    }

    /// Checks if the Task is Pending or Ready
    pub(crate) fn ready(self: &Arc<Task>) -> bool {
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
                let task = match receiver.recv() {
                    Ok(task_related) => task_related,
                    Err(e) => panic!("{}", e),
                };

                task.poll();
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
        let handle = TaskHandle::new(Arc::downgrade(&task));

        sender.send(task).expect("failed to send task to runtime!");

        handle
    }

    /// Register device in the I/O Reactor's registry
    /// Essentially it is just `Reactor::register`
    pub fn register(dev: &mut impl Source, interest: Interest) -> IoResult<()> {
        Reactor::register(dev, interest)
    }

    /// Reregister device in the I/O Reactor's registry
    /// Essentially it is just `Reactor::reregister`
    pub fn reregister(src: &mut impl Source, token: usize, interest: Interest) -> IoResult<()> {
        Reactor::reregister(src, token, interest)
    }

    /// Get registry.
    pub fn registry() -> &'static Registry {
        Runtime::get().handle.registry()
    }
}
