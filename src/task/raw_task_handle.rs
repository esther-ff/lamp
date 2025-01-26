use super::InnerTask;
use super::TaskHeader;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use std::ptr::NonNull;

pub struct RawTaskHandle<T: Future + Send + Sync + 'static> {
    ptr: NonNull<InnerTask<T>>,
}

impl<T: Future + Send + Sync + 'static> RawTaskHandle<T> {
    pub(crate) fn from_ptr(ptr: NonNull<TaskHeader>) -> RawTaskHandle<T> {
        RawTaskHandle {
            ptr: ptr.cast::<InnerTask<T>>(),
        }
    }

    pub(crate) fn read_output(&self, dst: &mut Poll<T::Output>) {
        use std::mem;

        // Safety: this function will not be called concurrently.
        // So the usage of a `MutCell<T>` and `as_ref()` is fine.
        let task = unsafe { self.ptr.as_ref() };
        let poll = unsafe { task.get_poll().get_mut() };

        let output = mem::replace(poll, Poll::Pending);
        let _ = mem::replace(dst, output); // ignored value
    }

    pub(crate) fn poll(&self) {
        // Safety: the pointer is valid.
        let task = unsafe { self.ptr.as_ref() };

        // Safety: this will be accessed from one thread
        let future = unsafe { task.get_future().get_mut() };

        let mut cx = Context::from_waker(task.waker());

        // Safety: `Future`s are !Unpin.
        let pin_future = unsafe { Pin::new_unchecked(future) };
        task.change_poll(pin_future.poll(&mut cx));
    }

    pub(crate) fn attach_waker(&self, waker: &Waker) {
        let task = unsafe { self.ptr.as_ref() };

        task.change_waker(waker);
    }
}
