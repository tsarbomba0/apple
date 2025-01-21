use crate::runtime::Task;
use crate::runtime::TaskRelated;
use futures::task::waker;
use futures::task::ArcWake;
use std::future::Future;
use std::pin::Pin;
use std::sync::mpmc::Sender;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

pub struct TaskHandle {
    task: Option<Arc<Task>>,
    exec: Sender<TaskRelated>,
}

impl TaskHandle {
    pub(crate) fn new(exec: Sender<TaskRelated>) -> TaskHandle {
        TaskHandle { task: None, exec }
    }

    pub(crate) fn attach_task(&mut self, task: Arc<Task>) {
        self.task = Some(task)
    }

    pub(crate) fn handle_poll(self: Arc<Self>) {
        let waker = waker(self.clone());
        let mut cx = Context::from_waker(&waker);
        let task_waker = self.task.as_ref().unwrap().get_waker();

        // Give to a task or replace it's current Waker
        let mut waker_field = task_waker.lock().expect("failed to lock task waker field!");
        *waker_field = Some(waker);

        // Poll the `Future` part of our struct.
    }

    fn send(self: &Arc<TaskHandle>) {
        match self.exec.send(TaskRelated::Handle(self.clone())) {
            Ok(_) => {}
            Err(e) => panic!("{:#}", e),
        }
    }
}

impl Future for TaskHandle {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.task.clone().unwrap().ready() {
            self.inner.clone().poll();
            return Poll::Pending;
        }
        Poll::Ready(())
    }
}

impl ArcWake for TaskHandle {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.send()
    }
}
