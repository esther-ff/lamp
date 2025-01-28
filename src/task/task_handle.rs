use super::RawTask;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Handle to a `Task` that can be `.await`ed.
pub struct TaskHandle<T> {
    raw: RawTask,
    _type: PhantomData<T>,
}

impl<T: Future> TaskHandle<T> {
    pub(crate) fn new(raw: RawTask) -> TaskHandle<T> {
        TaskHandle {
            raw,
            _type: PhantomData,
        }
    }
}

impl<T> Future for TaskHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut poll = Poll::Pending;

        self.raw.read_output(&mut poll as *mut _ as *mut ());

        if poll.is_pending() {
            self.raw.attach_handle_waker(cx.waker());
        }

        poll
    }
}
