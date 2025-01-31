use std::ptr::NonNull;
use std::task::{RawWaker, RawWakerVTable, Waker};

use super::task::Header;
use super::task::RawTask;

fn make_vtable() -> &'static RawWakerVTable {
    &RawWakerVTable::new(clone_fn, wake_fn, wake_by_ref_fn, drop_fn)
}

unsafe fn clone_fn(ptr: *const ()) -> RawWaker {
    let raw = RawTask::from_ptr(ptr as *mut Header);
    raw.ref_inc();

    RawWaker::new(ptr, make_vtable())
}

fn wake_fn(ptr: *const ()) {
    let raw = RawTask::from_ptr(ptr as *mut Header);
    raw.send_note();
}
fn wake_by_ref_fn(ptr: *const ()) {
    let raw = RawTask::from_ptr(ptr as *mut Header);
    raw.send_note();
}
fn drop_fn(ptr: *const ()) {
    let raw = RawTask::from_ptr(ptr as *mut Header);

    // this should be replaced
    if raw.ref_dec() == 0 {
        raw.destroy();
    }
}

pub(crate) fn make_waker(ptr: NonNull<Header>) -> Waker {
    let raw = RawTask::from_ptr(ptr.as_ptr());
    raw.ref_inc();

    let const_ptr = ptr.as_ptr() as *const ();
    unsafe { Waker::from_raw(RawWaker::new(const_ptr, make_vtable())) }
}
