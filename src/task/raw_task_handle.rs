use super::InnerTask;
use super::TaskHeader;
use std::pin::{Pin, pin};
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
        let poll = unsafe { (*self.ptr.as_ptr()).get_poll().get_mut() };

        let output = mem::replace(poll, Poll::Pending);
        let _ = mem::replace(dst, output); // ignored value
    }

    pub(crate) fn poll(&self) {
        // Safety:
        //
        // The pointer dereferenced for 1,2 and 3 is valid
        //
        // For #1 the Future will uphold the `Pin` contract.
        unsafe {
            let task = self.ptr.as_ptr();
            // #1
            let future = Pin::new_unchecked((*task).get_future().get_mut());
            // #2
            let mut cx = Context::from_waker((*task).get_waker());
            // #3
            (*task).change_poll(future.poll(&mut cx));
        };
    }

    pub(crate) fn attach_waker(&mut self, waker: &Waker) {
        unsafe { (*self.ptr.as_ptr()).set_waker(waker) }
    }

    pub(crate) fn wake_up_handle(&self) {
        unsafe { (*self.ptr.as_ptr()).wake_up_handle() };
    }

    pub(crate) fn attach_handle_waker(&self, waker: &Waker) {
        unsafe {
            (*self.ptr.as_ptr()).attach_handle_waker(waker);
        }
    }
}
