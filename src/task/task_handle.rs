use super::{RawTask, Task};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct TaskHandle<T> {
    task: Task,
    _type: PhantomData<T>,
}

impl<T> TaskHandle<T> {
    pub fn new(raw: RawTask) -> TaskHandle<T> {
        TaskHandle {
            task: Task::create_from_raw(raw),
            _type: PhantomData,
        }
    }

    pub fn raw(&self) -> RawTask {
        self.task.raw
    }
}

impl<T> Future for TaskHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut out = Poll::Pending;

        self.raw().read_output(&mut out as *mut _ as *mut ());

        if out.is_pending() {
            self.raw().set_handle_waker(cx.waker())
        }

        out
    }
}

impl<T> std::ops::Drop for TaskHandle<T> {
    fn drop(&mut self) {
        println!("[HANDLE] dropped handle")
    }
}

unsafe impl<T> Send for TaskHandle<T> {}
