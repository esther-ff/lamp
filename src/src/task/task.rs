use super::handle::TaskHandle;
use super::note::Note;
use super::vtable::{Vtable, vtable};
use super::waker;

use std::cell::{Cell, UnsafeCell};
use std::future::Future;
use std::ptr::NonNull;
use std::sync::atomic::AtomicU8;
use std::sync::mpsc::Sender;
use std::task::{Poll, Waker};

static REF_COUNT_BASE: u8 = 2;

/// Owned task struct.
pub struct Task {
    raw: RawTask,
}

impl Task {
    pub fn new<F: Future + Send + 'static>(
        f: F,
        id: u64,
        sender: Sender<Note>,
    ) -> (Task, Note, TaskHandle<F::Output>) {
        let raw = RawTask::new(f, sender);
        let waker = waker::make_waker(raw.ptr);

        raw.set_waker(Some(waker));

        (Task { raw }, Note(id), TaskHandle::new(raw))
    }

    pub(crate) fn poll(&self) -> bool {
        self.raw.poll()
    }

    pub(crate) fn destroy(&self) {
        self.raw.destroy()
    }
}

impl std::ops::Drop for Task {
    fn drop(&mut self) {
        self.raw.wake_handle();
        if self.raw.ref_dec() == 0 {
            println!("Deallocated the pointer");
            self.raw.destroy();
        }

        println!("Dropped task!");
    }
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

// Header, often used and updated data.
pub(crate) struct Header {
    pub(crate) id: u64,
    pub(crate) refs: AtomicU8,
    pub(crate) vtable: &'static Vtable,
    pub(crate) sender: Sender<Note>,
}

unsafe impl Send for Header {}

// Middle, contains the future and it's output.
pub(crate) struct Middle<F: Future + Send + 'static> {
    future: UnsafeCell<F>,
    pub(crate) poll: UnsafeCell<Poll<F::Output>>,
}

// Tail of the task, used for rarely updated data
pub(crate) struct Tail {
    pub(crate) waker: UnsafeCell<Option<Waker>>,
    pub(crate) h_waker: Cell<Option<Waker>>,
}

impl Tail {
    // sets the waker for the task
    pub(crate) fn set_waker(&self, w: Option<Waker>) {
        unsafe { *(self.waker.get()) = w };
    }
}

pub(crate) struct Core<F: Future + Send + 'static> {
    head: Header,
    mid: Middle<F>,
    tail: Tail,
}

impl<F: Future + Send + 'static> Core<F> {
    pub(crate) fn new(f: F, id: u64, sender: Sender<Note>) -> Core<F> {
        let head = Header {
            id,
            refs: AtomicU8::new(REF_COUNT_BASE),
            vtable: vtable::<F>(),
            sender,
        };

        let mid = Middle {
            future: UnsafeCell::new(f),
            poll: UnsafeCell::new(Poll::Pending),
        };

        let tail = Tail {
            waker: UnsafeCell::new(None),
            h_waker: Cell::new(None),
        };

        Core { head, mid, tail }
    }

    pub(crate) fn header(&self) -> &Header {
        &self.head
    }

    pub(crate) fn middle(&self) -> &Middle<F> {
        &self.mid
    }

    pub(crate) fn tail(&self) -> &Tail {
        &self.tail
    }

    #[allow(clippy::mut_from_ref)]
    // This will be probably changed later.
    pub(crate) fn future(&self) -> &mut F {
        unsafe { &mut *self.mid.future.get() }
    }

    pub(crate) fn waker(&self) -> Option<&Waker> {
        match unsafe { &*self.tail.waker.get() } {
            None => None,
            Some(waker) => Some(&waker),
        }
    }

    // Set the waker used by the handle
    pub(crate) fn set_handle_waker(&self, waker: &Waker) {
        self.tail().h_waker.set(Some(waker.clone()))
    }

    // Getters for some specific stuff
    pub(crate) fn poll_output(&self) -> Poll<F::Output> {
        use std::mem;

        let reference = unsafe { &mut *self.mid.poll.get() };

        mem::replace(reference, Poll::Pending)
    }
}

unsafe impl<F: Send + Future> Send for Core<F> {}

#[derive(Copy, Clone)]
pub(crate) struct RawTask {
    ptr: NonNull<Header>,
}

impl RawTask {
    // creates a new raw task from a future
    pub fn new<F: Future + Send + 'static>(f: F, sender: Sender<Note>) -> RawTask {
        let ptr =
            unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(Core::new(f, 1, sender)))) };

        RawTask {
            ptr: ptr.cast::<Header>(),
        }
    }

    // creates a raw task from a pointer
    pub fn from_ptr(ptr: *mut Header) -> RawTask {
        RawTask {
            ptr: unsafe { NonNull::new_unchecked(ptr) },
        }
    }

    // convenience to access the header
    fn header(&self) -> &Header {
        unsafe { self.ptr.as_ref() }
    }

    // convenience to access the vtable
    fn vtable(&self) -> &Vtable {
        self.header().vtable
    }

    // Polls the future inside
    fn poll(self) -> bool {
        (self.vtable().poll)(self.ptr)
    }

    // Reviews the saved poll
    //
    // Writes it to the pointer and attaches the waker.
    pub(crate) fn review(self, dst: *const (), waker: &Waker) {
        (self.vtable().review)(self.ptr, dst, waker)
    }

    // Sends notification to the channel.
    pub(crate) fn send_note(self) {
        (self.vtable().send_note)(self.ptr)
    }

    // Sets the task's waker.
    pub(crate) fn set_waker(self, waker: Option<Waker>) {
        (self.vtable().set_waker)(self.ptr, waker);
    }

    // Wakes up the handle's waker (if there is one).
    pub(crate) fn wake_handle(self) {
        (self.vtable().wake_handle)(self.ptr)
    }

    // Deallocates the pointer used for handling the task.
    pub(crate) fn destroy(self) {
        (self.vtable().destroy)(self.ptr)
    }

    // Returns ref count after subtrackting.
    pub(crate) fn ref_dec(self) -> u8 {
        (self.vtable().ref_dec)(self.ptr)
    }

    // Returns ref count after adding
    pub(crate) fn ref_inc(self) -> u8 {
        (self.vtable().ref_inc)(self.ptr)
    }
}
