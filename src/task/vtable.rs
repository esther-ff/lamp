use crate::task::RawTaskHandle;
use crate::task::TaskHeader;
use std::future::Future;
use std::ptr::NonNull;
use std::sync::atomic::Ordering;
use std::task::{Poll, Waker};

pub struct Vtable {
    pub poll: fn(NonNull<TaskHeader>),
    pub wake_up_handle: fn(NonNull<TaskHeader>),
    pub attach_handle_waker: fn(NonNull<TaskHeader>, &Waker),
    pub read_output: fn(NonNull<TaskHeader>, *mut ()),
    pub set_waker: fn(NonNull<TaskHeader>, &Waker),
    pub ref_increase: fn(NonNull<TaskHeader>) -> u8,
    pub ref_decrease: fn(NonNull<TaskHeader>) -> u8,
    pub deallocate: fn(NonNull<TaskHeader>),
}

impl Vtable {
    pub(crate) fn new<F: Future + 'static + Send + Sync>() -> Vtable {
        Vtable {
            poll: poll::<F>,
            read_output: read_output::<F>,
            attach_handle_waker: attach_handle_waker::<F>,
            wake_up_handle: wake_up_handle::<F>,
            set_waker: set_waker::<F>,
            ref_increase,
            ref_decrease,
            deallocate,
        }
    }
}

fn poll<F: Future + Send + Sync + 'static>(ptr: NonNull<TaskHeader>) {
    let handle: RawTaskHandle<F> = RawTaskHandle::from_ptr(ptr);

    handle.poll()
}

fn read_output<F: Future + Send + Sync + 'static>(ptr: NonNull<TaskHeader>, ptr1: *mut ()) {
    let dst = unsafe { &mut *(ptr1 as *mut Poll<F::Output>) };
    let handle: RawTaskHandle<F> = RawTaskHandle::from_ptr(ptr);

    handle.read_output(dst);
}

fn set_waker<F: Future + Send + Sync + 'static>(ptr: NonNull<TaskHeader>, waker: &Waker) {
    let mut handle: RawTaskHandle<F> = RawTaskHandle::from_ptr(ptr);

    handle.attach_waker(waker)
}

fn wake_up_handle<F: Future + Send + Sync + 'static>(ptr: NonNull<TaskHeader>) {
    let handle: RawTaskHandle<F> = RawTaskHandle::from_ptr(ptr);

    handle.wake_up_handle();
}

fn attach_handle_waker<F: Future + Send + Sync + 'static>(ptr: NonNull<TaskHeader>, waker: &Waker) {
    let handle: RawTaskHandle<F> = RawTaskHandle::from_ptr(ptr);

    handle.attach_handle_waker(waker)
}

fn ref_increase(ptr: NonNull<TaskHeader>) -> u8 {
    let header = unsafe { ptr.as_ref() };
    header.count.fetch_add(1, Ordering::SeqCst)
}

fn ref_decrease(ptr: NonNull<TaskHeader>) -> u8 {
    let header = unsafe { ptr.as_ref() };
    header.count.fetch_sub(1, Ordering::SeqCst)
}

// Deallocate the pointer
// Only use if the ref count is 0
fn deallocate(mut ptr: NonNull<TaskHeader>) {
    let mut_ptr = unsafe { ptr.as_mut() };
    drop(unsafe { Box::from_raw(mut_ptr) })
}
