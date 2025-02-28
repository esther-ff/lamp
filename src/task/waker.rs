use std::ptr::NonNull;
use std::task::{RawWaker, RawWakerVTable, Waker};

use super::task::Header;
use super::task::RawTask;

use log::warn;

fn make_vtable() -> &'static RawWakerVTable {
    &RawWakerVTable::new(clone_fn, wake_fn, wake_by_ref_fn, drop_fn)
}

unsafe fn clone_fn(ptr: *const ()) -> RawWaker {
    let raw = RawTask::from_ptr(ptr as *mut Header);
    warn!("Incremented in waker! (clone_fn)");
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
    warn!("Decremented in waker! (drop_fn) ");
    raw.ref_destroy();
}

pub(crate) fn make_waker(ptr: NonNull<Header>) -> Waker {
    let raw = RawTask::from_ptr(ptr.as_ptr());
    warn!("Incremented in waker! (make_waker)");
    raw.ref_inc();

    let const_ptr = ptr.as_ptr() as *const ();
    unsafe { Waker::from_raw(RawWaker::new(const_ptr, make_vtable())) }
}
