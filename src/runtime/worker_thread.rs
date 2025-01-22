use crate::runtime::runtime::Task;
use slab::Slab;

use std::any::Any;
use std::cell::UnsafeCell;
use std::io;
use std::marker::PhantomData;
use std::sync::{mpsc, Arc};
use std::thread::{self, available_parallelism};

pub struct WorkerThread {
    name: Option<String>,
    sender: mpsc::Sender<Arc<Task>>,
    amount: usize,
    id: usize,
    active: bool,
    _marker: PhantomData<UnsafeCell<()>>,
}

impl WorkerThread {
    /// Creates one `WorkerThread`
    pub(crate) fn new(id: usize, name: Option<&str>) -> io::Result<WorkerThread> {
        let (sender, recv) = mpsc::channel::<Arc<Task>>();

        let _thread = thread::Builder::new();
        if let Some(name) = name {
            _thread.name(name.to_string())
        }

        _thread.no_hooks().spawn(move || {
            while let Ok(task) = recv.recv() {
                task.poll();
            }
        });

        Ok(WorkerThread {
            name,
            sender,
            amount: 0,
            id,
            active: true,
            _marker: PhantomData,
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

        for (key, entry) in thread_pool.iter_mut() {
            let name = Some(format!("thread-{}", key));
            *entry = WorkerThread::new(key, name).expect("Failed to create a thread!");
        }

        thread_pool
    }

    /// Checks if the specified amount of would-be used threads is possible to execute.
    pub fn ok_thread_amount(amount: usize) -> bool {
        available_parallelism().unwrap().get() >= amount
    }

    pub fn send(&mut self, task: &Arc<Task>) {
        self.sender.send(Arc::clone(task));
        self.amount += 1;
    }
}
