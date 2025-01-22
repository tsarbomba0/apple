use crate::runtime::WorkerThread;

struct ThreadPool {
    threads: Slab<WorkerThread>,
}

impl ThreadPool {
    /// Creates a new ThreadPool.
    pub fn new(num: usize) {
        ThreadPool {
            threads: WorkerThread::create_n(num),
        }
    }

    pub fn distribute_task(&self) {
        self.
    }
}
