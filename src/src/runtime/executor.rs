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
    // Task queue
    storage: Mutex<HashMap<u64, Task>>,

    // Channels for the main thread.
    recv: RecvWrap<Note>,
    sender: mpsc::Sender<Note>,

    // Storage for the main task. (spawned by Executor::start)
    main: Mutex<Option<Task>>,

    // Channels for other threads.
    recv1: RecvWrap<Note>,
    sender1: mpsc::Sender<Note>,

    // task ids
    count: AtomicU64,
}

impl Executor {
    fn new() -> Executor {
        let (sender, r) = mpsc::channel();
        let recv = RecvWrap { inner: r };

        let (sender1, r1) = mpsc::channel();
        let recv1 = RecvWrap { inner: r1 };
        Executor {
            storage: Mutex::new(HashMap::with_capacity(4096)),
            main: Mutex::new(None),
            recv,
            sender,
            recv1,
            sender1,
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
        let exec = Executor::get();

        let num = exec.count.fetch_add(1, Ordering::SeqCst);
        let (task, note, handle) = Task::new(f, num, exec.sender1.clone());
        drop(handle);

        Executor::add_task(task, num);
        Executor::get().sender1.send(note).unwrap();

        thread::spawn(move || {
            while let Ok(n) = Executor::get().recv.inner.recv() {
                let mut storage = Executor::get().storage.lock().unwrap();

                let task = storage.remove(&n.0).expect("no task with that id");
                let state = task.poll();

                if !state {
                    println!("Task not ready!");
                    let mut st = Executor::get().storage.lock().unwrap();
                    st.insert(n.0, task);
                } else {
                    println!("thread: polled task is ready!");
                };
            }
        });
        loop {
            while let Ok(n) = Executor::get().recv1.inner.recv() {
                let mut storage = Executor::get().storage.lock().unwrap();

                let task = storage.remove(&n.0).expect("no main task");
                drop(storage);
                let state = task.poll();
            }
        }
    }
    pub fn spawn<F>(f: F) -> TaskHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let exec = Executor::get();

        let num = exec.count.fetch_add(1, Ordering::SeqCst);
        let (task, note, handle) = Task::new(f, num, exec.sender.clone());
        Executor::add_task(task, note.0);

        println!("runtime: registered task with id: {}", &note.0);
        exec.sender.send(note).unwrap();
        handle
    }
}

// loop {
//     while let Ok(n) = Executor::get().recv.inner.recv() {
//         println!("runtime: polling task with id: {0}", n.0);

//         drop(storage);
//     }
