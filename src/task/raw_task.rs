use super::InnerTask;
use super::TaskHeader;

use std::ptr::NonNull;
use std::task::Waker;

pub struct RawTask {
    raw: NonNull<TaskHeader>,
}

impl RawTask {
    pub(crate) fn new<F: Future + Send + Sync + 'static>(future: F) -> RawTask {
        let ptr = Box::into_raw(Box::new(InnerTask::new(future)));

        RawTask {
            raw: unsafe { NonNull::new_unchecked(ptr).cast() },
        }
    }

    pub(crate) fn poll(&self) {
        println!("[RAW_TASK] polled task!");
        let vtable = unsafe { self.raw.as_ref().vtable() };
        (vtable.poll)(self.raw)
    }

    pub(crate) fn read_output(&self, dst: *mut ()) {
        let vtable = unsafe { self.raw.as_ref().vtable() };
        (vtable.read_output)(self.raw, dst)
    }

    pub(crate) fn attach_waker(&self, waker: &Waker) {
        let vtable = unsafe { self.raw.as_ref().vtable() };
        (vtable.attach_waker)(self.raw, waker)
    }

    pub(crate) fn ref_dec(&self) -> u8 {
        let vtable = unsafe { self.raw.as_ref().vtable() };
        (vtable.ref_dec)(self.raw)
    }

    pub(crate) fn ref_inc(&self) -> u8 {
        let vtable = unsafe { self.raw.as_ref().vtable() };
        (vtable.ref_inc)(self.raw)
    }

    pub(crate) fn dealloc(&self) {
        let vtable = unsafe { self.raw.as_ref().vtable() };
        (vtable.dealloc)(self.raw)
    }
}

impl std::clone::Clone for RawTask {
    fn clone(&self) -> RawTask {
        *self
    }
}

impl Copy for RawTask {}
