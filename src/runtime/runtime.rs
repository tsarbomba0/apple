use crate::io::Handle;
use crate::io::Reactor;
use crate::runtime::MutCell;
use crate::runtime::TaskHandle;
use crate::runtime::ThreadPool;

use async_lock::OnceCell;
use futures::task::{self, ArcWake};

use mio::event::Source;
use mio::{Interest, Registry};

use std::future::Future;
use std::pin::Pin;

use std::sync::mpmc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use std::io::Result as IoResult;

// Convenience type for the Futures used by the Runtime.
type RuntimeFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// Trait for `FutureTask` so we can use different result values.
trait FutureTaskTrait {
    fn poll(&mut self, cx: &mut Context<'_>);
    fn current_poll(&self) -> Poll<()>;
}

/// Underlying task for the `Task` struct.
struct FutureTask<T> {
    poll: Poll<()>,
    ft: RuntimeFuture<T>,
}

impl<T> FutureTask<T> {
    /// Creates a new `FutureTask` with a type of `RuntimeFuture`
    /// `RuntimeFuture` is a type alias referring to `Pin<Box<dyn Future<Output = ()> + Send`
    fn new(f: RuntimeFuture<T>) -> FutureTask<T> {
        FutureTask {
            poll: Poll::Pending,
            ft: f,
        }
    }
}

impl<T> FutureTaskTrait for FutureTask<T> {
    /// Polls the `Future` inside the `FutureTask`
    /// Only polls it if the poll returned earlier was `Poll::Pending`
    fn poll(&mut self, cx: &mut Context<'_>) {
        if self.poll.is_pending() {
            let poll = self.ft.as_mut().poll(cx);
            self.poll = match poll.is_ready() {
                true => Poll::Ready(()),
                false => Poll::Pending,
            }
        }
    }

    fn current_poll(&self) -> Poll<()> {
        self.poll
    }
}

/// Task struct used by the Runtime
pub struct Task {
    taskft: MutCell<Box<dyn FutureTaskTrait + 'static>>,
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
    fn new(tsft: Box<dyn FutureTaskTrait + 'static>, exec: Sender<Arc<Task>>) -> Task {
        let taskft = unsafe { MutCell::new(tsft) };
        Task {
            taskft,
            exec,
            waker: unsafe { MutCell::new(Waker::noop().clone()) },
        }
    }

    /// Convenience function to create a `Arc<Task>` from a type implementing `Future<Output = ()> + Send + 'static`
    fn arc_new<F, T: 'static>(future: F, sender: Sender<Arc<Task>>) -> Arc<Task>
    where
        F: Future<Output = T> + Send + 'static,
    {
        let task = Task::new(Box::new(FutureTask::new(Box::pin(future))), sender.clone());

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
        if self.taskft.current_poll().is_pending() {
            let waker = task::waker(self.clone());
            let mut cx = Context::from_waker(&waker);

            // Safety:
            // This will let only 1 thread access this mutably.
            unsafe { self.taskft.get_mut().poll(&mut cx) };
        }
    }

    /// Checks if the Task is Pending or Ready
    pub(crate) fn ready(self: &Arc<Task>) -> bool {
        !self.taskft.current_poll().is_pending()
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

    /// Thread pool.
    pool: Mutex<ThreadPool>,
}

// Global variable to hold the Runtime.
static RUNTIME: OnceCell<Runtime> = OnceCell::new();

impl Runtime {
    /// Builds the global Runtime instance.
    pub fn build(threads: usize) -> &'static Runtime {
        // Sender and Receiver channel for the Tasks
        let (sd, rc) = mpmc::channel();

        // Acquires the reference to the OnceCell<T> in the static variable
        // and initializes it with the Runtime
        RUNTIME.get_or_init_blocking(|| {
            // Driver and it's handle
            let driver = Reactor::new();

            // Runtime
            Runtime {
                receiver: rc,
                sender: sd,

                driver: driver.0,
                handle: Arc::clone(&driver.1),

                pool: Mutex::new(ThreadPool::new(threads)),
            }
        })
    }

    /// Obtains a immutable reference to the Runtime.
    ///
    /// Panics if there is no runtime present.
    pub fn get() -> &'static Runtime {
        RUNTIME.get().expect("There is no runtime available!")
    }

    /// Initializes the runtime with a Future created from the `main` function.
    pub fn init<F>(future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let sender = &Runtime::get().sender;
        let task = Task::arc_new(future, sender.clone());

        sender.send(task).expect("failed to initialize runtime");

        let receiver = &Runtime::get().receiver;
        loop {
            let task = match receiver.recv() {
                Ok(t) => t,
                Err(e) => panic!("{}", e),
            };

            task.poll();
        }
    }

    /// Spawns a task onto the Runtime.
    pub fn spawn<F, T: 'static>(future: F) -> TaskHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
    {
        let sender = Runtime::get().sender.clone();
        let task = Task::arc_new(future, sender);
        let handle = TaskHandle::new(Arc::downgrade(&task));

        match Runtime::send_task(task) {
            Ok(()) => {}
            Err(e) => panic!("{e}"),
        };

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

    /// Send a task to the thread pool.
    fn send_task(task: Arc<Task>) -> IoResult<()> {
        Runtime::get()
            .pool
            .lock()
            .expect("Failed lock on mutex containing the thread pool")
            .distribute_task(task)
    }
}
