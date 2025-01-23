use crate::runtime::{runtime::Task, WorkerThread};
use slab::Slab;
use std::io::Result as IoResult;
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

    /// This function will attempt to distribute tasks across the `WorkerThread`s
    /// An error returned from this function is probably very critical as it is related to
    /// a problem with recreating a thread.
    pub fn distribute_task(&mut self, task: Arc<Task>) -> IoResult<()> {
        let mut amnt_key = (0, 0);

        for (key, thread) in self.threads.iter_mut() {
            if thread.get_amount_sent() == 0 {
                amnt_key.1 = key;
                break;
            }
            if thread.get_amount_sent() > amnt_key.0 {
                amnt_key.1 = key;
                amnt_key.0 = thread.get_amount_sent();
            }
        }

        // We are 99% sure this should be filled
        // So `.expect` should never panic.
        let thread = self
            .threads
            .get_mut(amnt_key.1)
            .expect("This should NOT happen.");

        match thread.send(task) {
            Ok(()) => {
                println!(
                    "Sending task to thread: {} with count of {}",
                    thread.get_name(),
                    thread.get_amount_sent()
                );
            }
            Err(e) => {
                // log the error ofc
                println!("Error while sending to {0}: {1}", thread.get_name(), e);

                // if this throws an error
                // it's seriously bad
                thread.recreate_thread()?;
            }
        };

        Ok(())
    }
}
