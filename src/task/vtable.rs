use super::task::Header;
use crate::task::mantle::Mantle;
use std::future::Future;
use std::ptr::NonNull;
use std::sync::atomic::Ordering;
use std::task::Waker;

use log::info;

type Ptr = NonNull<Header>;

pub(crate) struct Vtable {
    pub(crate) poll: fn(Ptr) -> bool,
    pub(crate) review: fn(Ptr, *const (), &Waker),
    pub(crate) wake_handle: fn(Ptr),
    pub(crate) send_note: fn(Ptr),
    pub(crate) set_waker: fn(Ptr, Option<Waker>),
    pub(crate) ref_dec: fn(Ptr) -> u8,
    pub(crate) ref_inc: fn(Ptr) -> u8,
    pub(crate) destroy: fn(Ptr),
}

pub(crate) fn vtable<F: Future + Send + 'static>() -> &'static Vtable {
    &Vtable {
        poll: poll::<F>,
        destroy: destroy::<F>,
        review: review::<F>,
        wake_handle: wake_handle::<F>,
        send_note: send_note::<F>,
        set_waker: set_waker::<F>,
        ref_dec,
        ref_inc,
    }
}

fn poll<F: Future + Send + 'static>(ptr: Ptr) -> bool {
    let m: Mantle<F> = Mantle::from_raw(ptr);
    m.poll()
}

fn destroy<F: Future + Send + 'static>(ptr: Ptr) {
    let m: Mantle<F> = Mantle::from_raw(ptr);
    m.destroy();
    info!("destroyed the task pointer")
}
fn review<F: Future + Send + 'static>(ptr: Ptr, dst: *const (), waker: &Waker) {
    let m: Mantle<F> = Mantle::from_raw(ptr);
    m.review(dst, waker);
}

fn wake_handle<F: Future + Send + 'static>(ptr: Ptr) {
    let m: Mantle<F> = Mantle::from_raw(ptr);

    m.wake_handle();
}

fn send_note<F: Future + Send + 'static>(ptr: Ptr) {
    let m: Mantle<F> = Mantle::from_raw(ptr);

    m.send_note();
}

fn set_waker<F: Future + Send + 'static>(ptr: Ptr, waker: Option<Waker>) {
    let m: Mantle<F> = Mantle::from_raw(ptr);

    m.set_waker(waker);
}

fn ref_dec(ptr: Ptr) -> u8 {
    let output = unsafe { (*ptr.as_ptr()).refs.fetch_sub(1, Ordering::SeqCst) };

    let _id = unsafe { (*ptr.as_ptr()).id };
    output - 1
}
fn ref_inc(ptr: Ptr) -> u8 {
    let output = unsafe { (*ptr.as_ptr()).refs.fetch_add(1, Ordering::SeqCst) };

    let _id = unsafe { (*ptr.as_ptr()).id };
    output + 1
}
