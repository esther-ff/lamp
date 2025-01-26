#![allow(dead_code)]

use super::Notification;
use std::mem;
use std::sync::Arc;
use std::task::{RawWaker, RawWakerVTable, Waker};

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

// VTable functions
// Clone the Waker.
unsafe fn clone(data: *const ()) -> RawWaker {
    let arc = unsafe { Arc::from_raw(data as *const Notification) };
    let ptr = Arc::clone(&arc);
    mem::forget(arc);

    let raw_ptr = Arc::into_raw(ptr) as *const ();

    RawWaker::new(raw_ptr, &VTABLE)
}

// Drop the waker
unsafe fn drop(data: *const ()) {
    unsafe { Arc::from_raw(data as *const Notification) };
}

// Wake through the waker
unsafe fn wake(data: *const ()) {
    println!("Wake called!");
    let arc = unsafe { Arc::from_raw(data as *const Notification) };
    arc.send();
}

// Wake by reference using the waker
unsafe fn wake_by_ref(data: *const ()) {
    println!("Wake by ref called!");
    let arc = unsafe { Arc::from_raw(data as *const Notification) };
    let clone = Arc::clone(&arc);
    clone.send();
}

/// This function will provide us with a waker for a `Notification` assigned to a `Task`.
pub(crate) fn make_waker(notif: Arc<Notification>) -> Waker {
    let raw_waker = RawWaker::new(Arc::into_raw(notif) as *const (), &VTABLE);
    unsafe { Waker::from_raw(raw_waker) }
}
