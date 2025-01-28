use super::MutCell;
use super::Notification;
use super::Vtable;

use crate::task::RawTask;

use std::sync::{Arc, atomic, mpsc};
use std::task::{Poll, Waker};

pub struct Task {
    pub(crate) raw: RawTask,
}

impl Task {
    pub(crate) fn new<F: Future + Send + Sync + 'static>(
        future: F,
        chan: mpsc::Sender<Arc<Notification>>,
    ) -> (Task, Notification) {
        let raw = RawTask::new(future);
        let task = Task { raw };

        let notif = Notification::new(Task { raw }, chan);

        (task, notif)
    }

    pub(crate) fn poll(&self) {
        println!("[TASK] polled task!");
        self.raw.poll();
    }

    pub(crate) fn attach_waker(&self, waker: &Waker) {
        self.raw.attach_waker(waker);
    }
}

impl std::ops::Drop for Task {
    fn drop(&mut self) {
        if self.raw.ref_dec() == 0 {
            println!("[TASK] dropped task");
            self.raw.dealloc();
        }
    }
}

// CORE
pub struct Core {
    vtable: Vtable,
}

impl Core {
    pub fn new<F: Future + 'static + Send + Sync>() -> Core {
        Core {
            vtable: Vtable::new::<F>(),
        }
    }
}

// Task
pub struct InnerTask<T: Future + 'static + Send + Sync> {
    core: Core,
    count: atomic::AtomicU8,
    future: MutCell<T>,
    waker: MutCell<Waker>,
    poll: MutCell<Poll<T::Output>>,
}

impl<T: Send + Sync + Future + 'static> std::ops::Drop for InnerTask<T> {
    fn drop(&mut self) {
        println!("[INNER TASK] dropped inner task");
    }
}

impl<F: Future + 'static + Send + Sync> InnerTask<F> {
    pub(crate) fn new(future: F) -> InnerTask<F> {
        InnerTask {
            core: Core::new::<F>(),
            count: atomic::AtomicU8::new(2),
            future: unsafe { MutCell::new(future) },
            waker: unsafe { MutCell::new(Waker::noop().clone()) },
            poll: unsafe { MutCell::new(Poll::Pending) },
        }
    }

    /// This should NOT be called concurrently.
    pub(crate) fn get_poll(&self) -> &MutCell<Poll<F::Output>> {
        &self.poll
    }

    pub(crate) unsafe fn get_future(&self) -> &MutCell<F> {
        &self.future
    }

    pub(crate) fn get_waker(&self) -> &Waker {
        self.waker.get()
    }

    pub(crate) fn change_poll(&self, poll: Poll<F::Output>) {
        // Safety: accessed from one thread.
        *unsafe { self.poll.get_mut() } = poll;
    }

    pub(crate) fn change_waker(&self, waker: &Waker) {
        if !waker.will_wake(&self.waker) {
            println!("[TASK] changed waker!");
            unsafe { *self.waker.get_mut() = waker.clone() }
        }
    }

    // Returns the value that is now in the `AtomicU8`
    pub(crate) fn ref_dec(&self) -> u8 {
        self.count.fetch_sub(1, atomic::Ordering::SeqCst) - 1
    }

    pub(crate) fn ref_inc(&self) -> u8 {
        self.count.fetch_add(1, atomic::Ordering::SeqCst) + 1
    }
}
pub struct TaskHeader {
    core: Core,
}

impl TaskHeader {
    pub fn vtable(&self) -> &Vtable {
        &self.core.vtable
    }
}
