use crate::task::handle::TaskHandle;
use crate::task::note::Note;
use crate::task::task::Task;
use std::sync::atomic::{AtomicU64, Ordering};

use std::collections::HashMap;
use std::mem;
use std::sync::{Mutex, OnceLock, mpsc};
use std::thread;

static EXEC: OnceLock<Executor> = OnceLock::new();
struct RecvWrap<T> {
    inner: mpsc::Receiver<T>,
}

unsafe impl<T> Send for RecvWrap<T> {}
unsafe impl<T> Sync for RecvWrap<T> {}

pub struct Executor {
    storage: Mutex<HashMap<u64, Task>>,
    recv: RecvWrap<Note>,
    sender: mpsc::Sender<Note>,
    count: AtomicU64,
}

impl Executor {
    fn new() -> Executor {
        let (sender, r) = mpsc::channel();
        let recv = RecvWrap { inner: r };
        Executor {
            storage: Mutex::new(HashMap::with_capacity(4096)),
            recv,
            sender,
            count: AtomicU64::new(0),
        }
    }

    pub fn build() {
        EXEC.get_or_init(Executor::new);
    }

    pub fn get() -> &'static Executor {
        EXEC.get().expect("runtime is not running")
    }

    fn add_task(task: Task, id: u64) {
        let mut storage = Executor::get().storage.lock().unwrap();

        storage.insert(id, task);
    }

    pub fn start<F>(f: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static + std::fmt::Debug,
    {
        let num = Executor::get().count.fetch_add(1, Ordering::SeqCst);
        let (task, note, _handle) = Task::new(f, num);

        Executor::add_task(task, note.0);
        Executor::get().sender.send(note).unwrap();
        println!("runtime: spawned main task with id: {} ", &note.0);
        //loop {

        loop {
            while let Ok(n) = Executor::get().recv.inner.recv() {
                println!("runtime: polling task with id: {0}", n.0);

                let mut storage = Executor::get().storage.lock().unwrap();

                //if n.0 != 1 {
                match storage.remove(&n.0) {
                    Some(task) => {
                        thread::spawn(move || {
                            println!("thread: thread received a task.");
                            if !task.poll() {
                                let mut st = Executor::get().storage.lock().unwrap();

                                st.insert(n.0, task);
                            } else {
                                println!("thread: polled task is ready!");
                            };
                        });
                    }
                    None => println!("runtime: no task of id: {}", &n.0),
                };
                //} else {
                //storage.get(&1).unwrap().poll();
                //sbin}
                drop(storage);
            }
        }
    }

    pub fn spawn<F>(f: F) -> TaskHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let num = Executor::get().count.fetch_add(1, Ordering::SeqCst);
        let (task, note, handle) = Task::new(f, num);
        Executor::add_task(task, note.0);

        println!("runtime: registered task with id: {}", &note.0);
        Executor::get().sender.send(note).unwrap();
        handle
    }
}
