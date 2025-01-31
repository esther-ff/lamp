// Mantle for the task.
use super::note::Note;
use super::task::{Core, Header};

use std::future::Future;
use std::pin::Pin;
use std::ptr::NonNull;
use std::task::{Context, Poll, Waker};

#[derive(Copy, Clone)]
pub(crate) struct Mantle<F: Future + Send + 'static> {
    ptr: NonNull<Core<F>>,
}

impl<F: Future + Send + 'static> Mantle<F> {
    pub(crate) fn from_raw(ptr: NonNull<Header>) -> Mantle<F> {
        Mantle {
            ptr: ptr.cast::<Core<F>>(),
        }
    }

    fn core(&self) -> &Core<F> {
        unsafe { self.ptr.as_ref() }
    }

    pub(crate) fn poll(self) -> bool {
        let future = unsafe { Pin::new_unchecked(self.core().future()) };

        let mut cx = Context::from_waker(self.core().waker().unwrap());
        let output = future.poll(&mut cx);
        let ready = output.is_ready();
        let field = unsafe { &mut *self.core().middle().poll.get() };
        *field = output;

        ready
    }

    pub(crate) fn destroy(self) {
        let val = unsafe { Box::from_raw(self.ptr.as_ptr()) };

        drop(val)
    }

    // Obtains the poll from the future.
    // writes it into the provided pointer.
    pub(crate) fn review(self, dst: *const (), waker: &Waker) {
        let dest = unsafe { &mut *(dst as *mut Poll<F::Output>) };
        *dest = self.core().poll_output();
        println!("runtime: transferred value to handle");
        self.core().set_handle_waker(waker);
    }

    // Wakes up the handle if there is a waker present.
    pub(crate) fn wake_handle(self) {
        let waker = self.core().tail().h_waker.replace(None);
        if let Some(w) = waker {
            println!("task: woke up waker");
            w.wake_by_ref()
        } else {
            println!("no waker!");
        }
    }

    // Sets the waker for a task.
    pub(crate) fn set_waker(self, waker: Option<Waker>) {
        self.core().tail().set_waker(waker);
    }

    // Sends a note to the assigned channel
    pub(crate) fn send_note(self) {
        let header = self.core().header();

        println!("header id: {}", header.id);
        header.sender.send(Note(header.id));
    }
}
