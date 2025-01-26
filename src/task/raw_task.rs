use super::RawTaskHandle;
use super::Task;
use super::TaskHeader;

use std::ptr::NonNull;

pub struct RawTask {
    raw: NonNull<TaskHeader>,
}

impl RawTask {
    pub(crate) fn new<F: Future + Send + Sync + 'static>(future: F) -> RawTask {
        let ptr = Box::into_raw(Box::new(Task::new(future, 0)));

        RawTask {
            raw: unsafe { NonNull::new_unchecked(ptr).cast() },
        }
    }

    pub(crate) fn poll(&self) {
        println!("Polling?!");
        let vtable = unsafe { self.raw.as_ref().vtable() };
        (vtable.poll)(self.raw)
    }

    pub(crate) fn read_output(&self, dst: *mut ()) {
        let vtable = unsafe { self.raw.as_ref().vtable() };
        (vtable.read_output)(self.raw, dst)
    }
}
