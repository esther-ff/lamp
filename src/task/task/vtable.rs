use crate::task::RawTaskHandle;
use crate::task::TaskHeader;
use std::future::Future;
use std::ptr::NonNull;
use std::task::{Poll, Waker};

pub struct Vtable {
    pub poll: fn(NonNull<TaskHeader>),
    pub read_output: fn(NonNull<TaskHeader>, *mut ()),
    pub attach_waker: fn(NonNull<TaskHeader>, &Waker),
    pub ref_dec: fn(NonNull<TaskHeader>) -> u8,
    pub ref_inc: fn(NonNull<TaskHeader>) -> u8,
    pub dealloc: fn(NonNull<TaskHeader>),
}

impl Vtable {
    pub(crate) fn new<F: Future + 'static + Send + Sync>() -> Vtable {
        Vtable {
            poll: poll::<F>,
            read_output: read_output::<F>,
            attach_waker: attach_waker::<F>,
            ref_dec: ref_dec::<F>,
            ref_inc: ref_inc::<F>,
            dealloc: dealloc::<F>,
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

fn attach_waker<F: Future + Send + Sync + 'static>(ptr: NonNull<TaskHeader>, waker: &Waker) {
    let handle: RawTaskHandle<F> = RawTaskHandle::from_ptr(ptr);

    handle.attach_waker(waker)
}

fn ref_dec<F: Future + Send + Sync + 'static>(ptr: NonNull<TaskHeader>) -> u8 {
    let handle: RawTaskHandle<F> = RawTaskHandle::from_ptr(ptr);

    unsafe { (*handle.get_task()).ref_dec() }
}

fn ref_inc<F: Future + Send + Sync + 'static>(ptr: NonNull<TaskHeader>) -> u8 {
    let handle: RawTaskHandle<F> = RawTaskHandle::from_ptr(ptr);

    unsafe { (*handle.get_task()).ref_inc() }
}

fn dealloc<F: Future + Send + Sync + 'static>(ptr: NonNull<TaskHeader>) {
    let handle: RawTaskHandle<F> = RawTaskHandle::from_ptr(ptr);

    handle.dealloc()
}

fn schedule(_ptr: NonNull<TaskHeader>) {}
