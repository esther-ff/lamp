use super::InnerTask;
use super::TaskHeader;

use std::clone::Clone;
use std::ops::Drop;
use std::ptr::NonNull;
use std::task::Waker;

pub struct RawTask {
    raw: NonNull<TaskHeader>,
}

impl RawTask {
    pub(crate) fn new<F: Future + Send + Sync + 'static>(future: F) -> RawTask {
        let ptr = Box::into_raw(Box::new(InnerTask::new(future, Waker::noop().clone())));

        RawTask {
            raw: unsafe { NonNull::new_unchecked(ptr).cast() },
        }
    }

    pub(crate) fn poll(&self) {
        println!("[RAW_TASK] polled task!");
        let vtable = unsafe { self.raw.as_ref().vtable() };
        (vtable.poll)(self.raw)
    }

    pub(crate) fn wake_up_handle(&self) {
        let vtable = unsafe { self.raw.as_ref().vtable() };
        (vtable.wake_up_handle)(self.raw)
    }

    pub(crate) fn attach_handle_waker(&self, waker: &Waker) {
        let vtable = unsafe { self.raw.as_ref().vtable() };
        (vtable.attach_handle_waker)(self.raw, waker)
    }

    pub(crate) fn read_output(&self, dst: *mut ()) {
        let vtable = unsafe { self.raw.as_ref().vtable() };
        (vtable.read_output)(self.raw, dst)
    }

    pub(crate) fn set_waker(&self, waker: &Waker) {
        let vtable = unsafe { self.raw.as_ref().vtable() };
        (vtable.set_waker)(self.raw, waker)
    }

    fn ref_increase(&self) -> u8 {
        let vtable = unsafe { self.raw.as_ref().vtable() };
        (vtable.ref_increase)(self.raw)
    }

    fn ref_decrease(&self) -> u8 {
        let vtable = unsafe { self.raw.as_ref().vtable() };
        (vtable.ref_decrease)(self.raw)
    }

    fn deallocate(&self) {
        let vtable = unsafe { self.raw.as_ref().vtable() };
        (vtable.deallocate)(self.raw)
    }
}

impl Clone for RawTask {
    fn clone(&self) -> RawTask {
        self.ref_increase();

        RawTask { raw: self.raw }
    }
}

impl Drop for RawTask {
    fn drop(&mut self) {
        println!("Dropping!");
        let count = self.ref_decrease() - 1;

        match count {
            // This is the last reference, which was dropped
            // we can safely deallocate
            0 => self.deallocate(),

            // The only reference left is now the task handle
            // this means both the notification and the task were dropped
            // therefore the task is ready and the task handle can be notified
            1 => self.wake_up_handle(),

            // Any other case
            _ => (),
        }
    }
}
