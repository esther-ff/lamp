use super::handle::TaskHandle;
use super::note::Note;
use super::vtable::{Vtable, vtable};
use super::waker;

use log::{info, warn};

use std::cell::UnsafeCell;
use std::future::Future;
use std::ptr::NonNull;
use std::sync::Mutex;
use std::sync::atomic::AtomicU8;
use std::sync::mpsc::Sender;
use std::task::{Poll, Waker};

static REF_COUNT_BASE: u8 = 0;

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
        let raw = RawTask::new(f, sender, id);
        warn!("Incremented in Task::new!");
        raw.ref_inc();
        let waker = waker::make_waker(raw.ptr);

        raw.set_waker(Some(waker));

        (Task { raw }, Note(id), TaskHandle::new(raw))
    }

    pub(crate) fn poll(&self) -> bool {
        self.raw.poll()
    }
}

impl std::ops::Drop for Task {
    fn drop(&mut self) {
        // Wakes up the handle.
        // This is here because the task is considered as finished
        // when it is dropped in the Executor's loop
        self.raw.wake_handle();

        // The cell containing the waker is set to None,
        // which drops the waker inside, which decreases our ref count.
        self.raw.set_waker(None);

        // This decreases the ref count
        // and destroys the pointer if it is 0.
        self.raw.ref_destroy();
    }
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

// Header, often used and updated data.
pub(crate) struct Header {
    // Id of the task.
    pub(crate) id: u64,

    // Number of references.
    pub(crate) refs: AtomicU8,

    // Virtual function table.
    pub(crate) vtable: &'static Vtable,

    // Channel to send Notifications.
    pub(crate) sender: Sender<Note>,
}

unsafe impl Send for Header {}

// Middle, contains the future and it's output.
pub(crate) struct Middle<F: Future + Send + 'static> {
    // Stored Future
    future: UnsafeCell<F>,

    // Previous poll output.
    pub(crate) poll: UnsafeCell<Poll<F::Output>>,
}

// Tail of the task, used for rarely updated data
pub(crate) struct Tail {
    // Waker for the task.
    pub(crate) waker: UnsafeCell<Option<Waker>>,

    // Waker for the task's handle.
    // TODO: find a way to do this lock-free.
    pub(crate) h_waker: Mutex<Option<Waker>>,
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
            h_waker: Mutex::new(None),
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
    /// Obtains a mutable reference to the Future inside
    pub(crate) fn future(&self) -> &mut F {
        unsafe { &mut *self.mid.future.get() }
    }

    /// Obtains a reference or None to the waker.
    pub(crate) fn waker(&self) -> Option<&Waker> {
        match unsafe { &*self.tail.waker.get() } {
            None => None,
            Some(waker) => Some(waker),
        }
    }

    /// Set the waker used by the handle
    pub(crate) fn set_handle_waker(&self, waker: &Waker) {
        *self.tail().h_waker.lock().unwrap() = Some(waker.clone())
    }

    /// Obtains the poll output.
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
    /// Creates a new raw task from a future
    pub fn new<F: Future + Send + 'static>(f: F, sender: Sender<Note>, id: u64) -> RawTask {
        let ptr =
            unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(Core::new(f, id, sender)))) };

        RawTask {
            ptr: ptr.cast::<Header>(),
        }
    }

    /// Convenience function.
    /// Reduces the reference count and if it's zero
    /// the heap pointer is deallocated.
    pub(crate) fn ref_destroy(self) {
        if self.ref_dec() == 0 {
            self.destroy();
        }
    }

    /// Creates a raw task from a pointer
    pub fn from_ptr(ptr: *mut Header) -> RawTask {
        RawTask {
            ptr: unsafe { NonNull::new_unchecked(ptr) },
        }
    }

    /// Convenience function to access the header
    pub(crate) fn header(&self) -> &Header {
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

    /// Reviews the saved poll
    ///
    /// Writes it to the pointer and attaches the waker.
    pub(crate) fn review(self, dst: *const (), waker: &Waker) {
        (self.vtable().review)(self.ptr, dst, waker)
    }

    /// Sends notification to the channel.
    pub(crate) fn send_note(self) {
        (self.vtable().send_note)(self.ptr)
    }

    /// Sets the task's waker.
    pub(crate) fn set_waker(self, waker: Option<Waker>) {
        (self.vtable().set_waker)(self.ptr, waker);
    }

    /// Wakes up the handle's waker (if there is one).
    pub(crate) fn wake_handle(self) {
        (self.vtable().wake_handle)(self.ptr)
    }

    /// Deallocates the pointer used for handling the task.
    pub(crate) fn destroy(self) {
        (self.vtable().destroy)(self.ptr)
    }

    /// Decreases the reference count by 1.
    /// Returns ref count after subtrackting.
    pub(crate) fn ref_dec(self) -> u8 {
        (self.vtable().ref_dec)(self.ptr)
    }

    /// Increases the reference count by 1.
    /// Returns reference count after adding
    pub(crate) fn ref_inc(self) -> u8 {
        (self.vtable().ref_inc)(self.ptr)
    }
}
