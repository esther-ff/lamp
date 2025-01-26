use crate::Notification;
use crate::Task;
use crate::runtime::reactor::{Handle, Reactor};
use crate::task::waker::make_waker;

use std::sync::{Arc, OnceLock, mpsc};

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

struct ReceiverWrapper<T> {
    receiver: mpsc::Receiver<T>,
}

impl<T> ReceiverWrapper<T> {
    fn receiver(&self) -> &mpsc::Receiver<T> {
        &self.receiver
    }
}

unsafe impl<T> Send for ReceiverWrapper<T> {}
unsafe impl<T> Sync for ReceiverWrapper<T> {}

pub struct Runtime {
    receiver: ReceiverWrapper<Arc<Notification>>,
    sender: mpsc::Sender<Arc<Notification>>,
    reactor: Reactor,
    handle: Arc<Handle>,
}

impl Runtime {
    fn new() -> Runtime {
        let (sender, receiver) = mpsc::channel();

        Runtime {
            receiver: ReceiverWrapper { receiver },
            sender,
            reactor: Reactor {},
            handle: Arc::new(Handle {}),
        }
    }

    pub fn create() {
        RUNTIME.get_or_init(Runtime::new);
    }

    pub fn init() {
        let receiver = &Runtime::get().receiver.receiver();

        while let Ok(notif) = receiver.recv() {
            println!("[RUNTIME] polled the task!");
            notif.test()
        }
    }

    pub fn get() -> &'static Runtime {
        RUNTIME.get().expect("no runtime present")
    }

    pub fn spawn<F>(future: F)
    where
        F: Future + Send + Sync + 'static,
    {
        let sender = &Runtime::get().sender;
        let (task, notif) = Task::new(future, sender.clone());
        let arc_notif = Arc::new(notif);
        task.attach_waker(&make_waker(arc_notif.clone()));

        sender.send(arc_notif).expect("failed send");
    }
}
