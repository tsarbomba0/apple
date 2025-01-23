use crate::runtime::Task;
use std::future::Future;
use std::pin::Pin;
use std::sync::Weak;
use std::task::{Context, Poll, Waker};

pub struct TaskHandle<T> {
    task: Weak<Task>,
}

impl TaskHandle {
    // No-operation context just to satisfy the `Future` trait.
    const CONTEXT_NOOP: Context<'_> = Context::from_waker(Waker::noop());

    pub(crate) fn new(task: Weak<Task>) -> TaskHandle {
        TaskHandle { task }
    }
}

impl Future for TaskHandle<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let arc_task = match self.task.upgrade() {
            Some(a) => a,
            None => return Poll::Ready(()),
        };

        if !arc_task.ready() {
            arc_task.change_waker(cx.waker());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
