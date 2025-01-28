use super::Task;
use std::sync::{Arc, mpsc};

pub struct Notification {
    task: Task,
    chan: mpsc::Sender<Arc<Notification>>,
}

impl Notification {
    pub fn new(task: Task, chan: mpsc::Sender<Arc<Notification>>) -> Notification {
        Notification { task, chan }
    }

    pub fn test(&self) {
        self.task.poll()
    }

    pub fn send(self: Arc<Self>) {
        self.chan.send(self.clone()).expect("channel send fail")
    }
}

unsafe impl Send for Notification {}
unsafe impl Sync for Notification {}

impl std::ops::Drop for Notification {
    fn drop(&mut self) {
        println!("[NOTIF] dropped notification")
    }
}
