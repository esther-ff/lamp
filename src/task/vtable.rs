use crate::task::RawTaskHandle;
use crate::task::Task;
use crate::task::TaskHeader;
use std::future::Future;
use std::ptr::NonNull;
use std::task::Poll;

pub struct Vtable {
    pub poll: fn(NonNull<TaskHeader>),
    pub schedule: fn(NonNull<TaskHeader>),
    pub read_output: fn(NonNull<TaskHeader>, *mut ()),
}

impl Vtable {
    pub(crate) fn new<F: Future + 'static + Send + Sync>() -> Vtable {
        Vtable {
            poll: poll::<F>,
            read_output: read_output::<F>,
            schedule,
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
fn schedule(ptr: NonNull<TaskHeader>) {}
