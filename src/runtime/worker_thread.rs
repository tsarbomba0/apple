use crate::runtime::runtime::Task;
use slab::Slab;
use std::io;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::{mpsc, Arc};
use std::thread::{self, available_parallelism};

/// Describes a thread of the Runtime.
pub struct WorkerThread {
    name: Option<String>,
    sender: mpsc::Sender<Arc<Task>>,
    amount: usize,
    id: usize,
    active: bool,
}

impl std::fmt::Debug for WorkerThread {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("WorkerThread")
            .field("name", &self.get_name())
            .field("sender", &self.sender)
            .field("amount", &self.amount)
            .field("id", &self.id)
            .field("active", &self.active)
            .finish()
    }
}

impl WorkerThread {
    /// Creates one `WorkerThread`
    pub(crate) fn new(id: usize, name: Option<String>) -> io::Result<WorkerThread> {
        let (sender, recv) = mpsc::channel::<Arc<Task>>();

        let _thread = if let Some(ref n) = name {
            thread::Builder::new().name(n.clone())
        } else {
            thread::Builder::new()
        };

        _thread.spawn(move || {
            while let Ok(task) = recv.recv() {
                task.poll();
            }
        })?;

        Ok(WorkerThread {
            name,
            sender,
            amount: 0,
            id,
            active: true,
        })
    }

    /// Creates `amount` of `WorkerThread`s stored in a `Slab`.
    ///
    /// If any of the created threads fail, this will panic.
    ///
    /// The naming scheme of the threads is `thread-<id>` (ex. thread-2).
    pub(crate) fn create_n(amount: usize) -> Slab<WorkerThread> {
        // Panics if the amount specified is larger than the amount of threads on the CPU.
        assert!(
            WorkerThread::ok_thread_amount(amount),
            "Too much threads assigned!"
        );

        let mut thread_pool = Slab::with_capacity(amount);
        loop {
            let len = thread_pool.len();
            if len == amount {
                break;
            }

            let name = Some(format!("thread-{len}"));
            let thread = WorkerThread::new(len, name).expect("Failed to create a thread!");

            thread_pool.insert(thread);
        }
        thread_pool
    }

    /// Remake a thread.
    /// Use only if your thread panicked.
    pub(crate) fn recreate_thread(&mut self) -> io::Result<()> {
        let (sender, recv) = mpsc::channel::<Arc<Task>>();

        let name = self
            .name
            .as_ref()
            .map_or("unnamed".to_string(), |s| s.clone());

        let _thread = thread::Builder::new().name(name).spawn(move || {
            while let Ok(task) = recv.recv() {
                task.poll();
            }
        })?;

        self.sender = sender;
        self.amount = 0;

        Ok(())
    }

    /// Checks if the specified amount of would-be used threads is possible to execute.
    pub fn ok_thread_amount(amount: usize) -> bool {
        available_parallelism().unwrap().get() >= amount
    }

    /// Send a `Arc<Task>` to the `WorkerThread`.
    pub fn send(&mut self, task: Arc<Task>) -> Result<(), mpsc::SendError<Arc<Task>>> {
        self.sender.send(task)?;
        self.amount += 1;

        Ok(())
    }

    /// Obtains the amount of tasks sent to the thread.
    pub fn get_amount_sent(&self) -> usize {
        self.amount
    }

    /// Obtains the name of the thread
    /// or the default "unnamed".
    pub fn get_name(&self) -> &str {
        self.name.as_ref().map_or("unnamed", |s| s)
    }
}

unsafe impl Send for WorkerThread {}
unsafe impl Sync for WorkerThread {}

impl UnwindSafe for WorkerThread {}
impl RefUnwindSafe for WorkerThread {}
