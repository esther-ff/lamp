use super::MutCell;
use super::Notification;
use super::Vtable;

use crate::task::RawTask;

use std::marker::PhantomData;
use std::sync::{Arc, atomic::AtomicU8, mpsc};
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
        println!("[TASK] polled task!");
        self.raw.poll();
    }

    pub(crate) fn set_waker(&self, waker: &Waker) {
        self.raw.set_waker(waker);
    }
}

// CORE
pub struct Core {
    vtable: Vtable,
    handle_waker: MutCell<Waker>,
}

impl Core {
    pub fn new<F: Future + 'static + Send + Sync>() -> Core {
        Core {
            vtable: Vtable::new::<F>(),
            handle_waker: unsafe { MutCell::new(Waker::noop().clone()) },
        }
    }
}

// Task
pub struct InnerTask<T: Future + 'static + Send + Sync> {
    core: Core,
    count: AtomicU8,
    future: MutCell<T>,
    waker: Waker,
    poll: MutCell<Poll<T::Output>>,
    _dst: PhantomData<()>,
}

impl<F: Future + 'static + Send + Sync> InnerTask<F> {
    pub(crate) fn new(future: F, waker: Waker) -> InnerTask<F> {
        InnerTask {
            core: Core::new::<F>(),
            count: AtomicU8::new(1),
            future: unsafe { MutCell::new(future) },
            waker,
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

    pub(crate) fn attach_handle_waker(&self, waker: &Waker) {
        if !waker.will_wake(&self.core.handle_waker) {
            *unsafe { self.core.handle_waker.get_mut() } = waker.clone()
        };
    }

    pub(crate) fn wake_up_handle(&self) {
        self.core.handle_waker.wake_by_ref();
    }

    pub(crate) fn get_waker(&self) -> &Waker {
        &self.waker
    }

    pub(crate) fn change_poll(&self, poll: Poll<F::Output>) {
        // Safety: accessed from one thread.
        *unsafe { self.poll.get_mut() } = poll;
    }

    pub(crate) fn set_waker(&mut self, waker: &Waker) {
        println!("Setting waker!");
        dbg!(&self.count);
        self.waker = waker.clone();
        println!("Set waker!");
    }
}
pub struct TaskHeader {
    core: Core,
    pub(crate) count: AtomicU8,
}

impl TaskHeader {
    pub fn vtable(&self) -> &Vtable {
        &self.core.vtable
    }
}
