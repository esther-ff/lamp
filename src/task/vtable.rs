use crate::task::RawTaskHandle;
use crate::task::TaskHeader;
use std::future::Future;
use std::ptr::NonNull;
use std::task::{Poll, Waker};

pub struct Vtable {
    pub poll: fn(NonNull<TaskHeader>),
    pub schedule: fn(NonNull<TaskHeader>),
    pub read_output: fn(NonNull<TaskHeader>, *mut ()),
    pub attach_waker: fn(NonNull<TaskHeader>, &Waker),
}

impl Vtable {
    pub(crate) fn new<F: Future + 'static + Send + Sync>() -> Vtable {
        Vtable {
            poll: poll::<F>,
            read_output: read_output::<F>,
            schedule,
            attach_waker: attach_waker::<F>,
        }
    }
}

fn poll<F: Future + Send + Sync + 'static>(ptr: NonNull<TaskHeader>) {
    let handle: RawTaskHandle<F> = RawTaskHandle::from_ptr(ptr);

    handle.poll()
}

// fn test_fake_typing<T: std::fmt::Debug>(ptr: NonNull<Header>) {
//     let casted = ptr.cast::<Core<T>>();
//     println!("haha!");
//     println!("{}", unsafe { &casted.as_ref().text });
//     println!("{:#?}", unsafe { casted.as_ref() })
// }

fn read_output<F: Future + Send + Sync + 'static>(ptr: NonNull<TaskHeader>, ptr1: *mut ()) {
    let dst = unsafe { &mut *(ptr1 as *mut Poll<F::Output>) };
    let handle: RawTaskHandle<F> = RawTaskHandle::from_ptr(ptr);

    handle.read_output(dst);
}

fn attach_waker<F: Future + Send + Sync + 'static>(ptr: NonNull<TaskHeader>, waker: &Waker) {
    let handle: RawTaskHandle<F> = RawTaskHandle::from_ptr(ptr);

    handle.attach_waker(waker)
}

fn schedule(ptr: NonNull<TaskHeader>) {}
