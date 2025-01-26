use super::MutCell;
use super::Notification;
use super::Vtable;

use crate::task::RawTask;

use std::marker::PhantomData;
use std::sync::{Arc, mpsc};
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
        let task = Task { raw: raw.clone() };

        let notif = Notification::new(Task { raw }, chan);

        (task, notif)
    }

    pub(crate) fn poll(&self) {
        self.raw.poll();
    }

    pub(crate) fn attach_waker(&self, waker: &Waker) {
        self.raw.attach_waker(waker);
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
    future: MutCell<T>,
    waker: MutCell<Waker>,
    id: u64,
    poll: MutCell<Poll<T::Output>>,
    _dst: PhantomData<()>,
}

impl<F: Future + 'static + Send + Sync> InnerTask<F> {
    pub(crate) fn new(future: F, id: u64) -> InnerTask<F> {
        InnerTask {
            future: unsafe { MutCell::new(future) },
            core: Core::new::<F>(),
            waker: unsafe { MutCell::new(Waker::noop().clone()) },
            id,
            poll: unsafe { MutCell::new(Poll::Pending) },
            _dst: PhantomData,
        }
    }

    /// This should NOT be called concurrently.
    pub(crate) fn get_poll(&self) -> &MutCell<Poll<F::Output>> {
        &self.poll
    }

    pub(crate) unsafe fn get_future(&self) -> &MutCell<F> {
        &self.future
    }

    pub(crate) fn waker(&self) -> &Waker {
        self.waker.get()
    }

    pub(crate) fn change_poll(&self, poll: Poll<F::Output>) {
        // Safety: accessed from one thread.
        *unsafe { self.poll.get_mut() } = poll;
    }

    pub(crate) fn change_waker(&self, waker: &Waker) {
        println!("Waker change fn");
        if !waker.will_wake(&self.waker) {
            println!("Changed waker!");
            unsafe { *self.waker.get_mut() = waker.clone() }
        }
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
