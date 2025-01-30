use std::task::{RawWaker, Waker, RawWakerVTable};
use std::ptr::NonNull;

use super::task::Header;
use super::mantle::Mantle;

fn make_vtable() -> &'static RawWakerVTable {
    &RawWakerVTable::new(
        clone_fn
        wake_fn
        wake_by_ref_fn,
        drop_fn,
    )
}

fn clone_fn(ptr: NonNull<Header>) {}
fn wake_fn(ptr: NonNull<Header>) {}
fn wake_by_ref_fn(ptr: NonNull<Header>) {}
fn drop_fn(ptr: NonNull<Header>) {}


pub(crate) fn make_waker(ptr: NonNull<Header>) {
    RawWaker::new(ptr, make_vtable())
}
