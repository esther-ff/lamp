use super::task::RawTask;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct TaskHandle<T> {
    raw: RawTask,
    _t: PhantomData<T>,
}

impl<T> TaskHandle<T> {
    pub(crate) fn new(raw: RawTask) -> TaskHandle<T> {
        TaskHandle {
            raw,
            _t: PhantomData,
        }
    }
}

impl<T: std::fmt::Debug> Future for TaskHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut out = Poll::Pending;

        self.raw.review(&mut out as *mut _ as *const (), cx.waker());
        dbg!(&out);
        out
    }
}

impl<T> std::ops::Drop for TaskHandle<T> {
    fn drop(&mut self) {
        println!("Dropped handle!");
        if self.raw.ref_dec() == 0 {
            println!("Deallocated the pointer (handle)");
            self.raw.destroy();
        }
    }
}

unsafe impl<T> Send for TaskHandle<T> {}
