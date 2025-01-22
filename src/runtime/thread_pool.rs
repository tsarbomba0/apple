use crate::runtime::{runtime::Task, WorkerThread};
use slab::Slab;
use std::sync::Arc;

pub struct ThreadPool {
    threads: Slab<WorkerThread>,
}

impl ThreadPool {
    /// Creates a new ThreadPool.
    pub fn new(num: usize) -> ThreadPool {
        ThreadPool {
            threads: WorkerThread::create_n(num),
        }
    }

    pub fn distribute_task(&mut self, task: Arc<Task>) {
        let mut amnt_key = (0, 0);
        
        for (key, thread) in self.threads.iter_mut() {
            /// insert cool algorithm
        }
    }
}
