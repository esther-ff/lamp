mod task;
use std::future::Future;
use std::pin::Pin;
use std::ptr::NonNull;
use std::task::{Context, Poll};
use task::task::UpperTask;

struct Dummy;

impl Future for Dummy {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        println!("Ready!");
        Poll::Ready(())
    }
}

fn main() {
    println!("Okay start!");

    let test = Dummy;

    let task = UpperTask::new(test);

    task.poll();

    let mut poll: Poll<()> = Poll::Pending;

    task.raw.read_output(&mut poll as *mut _ as *mut ());

    dbg!(poll.is_ready());
}
