use super::MutCell;
use super::Vtable;
use crate::task::RawTask;

use futures::task::{self, ArcWake};
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

// temp. UpperTask
pub struct UpperTask {
    pub(crate) raw: RawTask,
}

impl UpperTask {
    pub(crate) fn new<F: Future + Send + Sync + 'static>(future: F) -> UpperTask {
        UpperTask {
            raw: RawTask::new(future),
        }
    }

    pub(crate) fn poll(&self) {
        self.raw.poll();
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

// RuntimeMessage
pub enum RuntimeMessage {
    TaskDone(u64),
    Pending(u64),
}

impl ArcWake for RuntimeMessage {
    fn wake_by_ref(arc_self: &Arc<RuntimeMessage>) {
        // Somehow notify runtime
    }
}

// Task
pub struct Task<T: Future + 'static + Send + Sync> {
    core: Core,
    future: MutCell<T>,
    waker: MutCell<Waker>,
    id: u64,
    poll: MutCell<Poll<T::Output>>,
    _dst: PhantomData<()>,
}

impl<F: Future + 'static + Send + Sync> Task<F> {
    pub(crate) fn new(future: F, id: u64) -> Task<F> {
        Task {
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

    pub(crate) fn change_poll(&self, poll: Poll<F::Output>) {
        // Safety: accessed from one thread.
        *unsafe { self.poll.get_mut() } = poll;
    }

    pub(crate) fn change_waker(&self, waker: &Waker) {
        if !waker.will_wake(&self.waker) {
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
