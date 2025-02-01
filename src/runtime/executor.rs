use crate::task::handle::TaskHandle;
use crate::task::note::Note;
use crate::task::task::Task;
use std::sync::atomic::{AtomicU64, Ordering};

use std::collections::HashMap;
use std::mem;
use std::sync::{Mutex, OnceLock, mpsc};
use std::thread;

static EXEC: OnceLock<Executor> = OnceLock::new();

struct ChannelPair<T> {
    s: mpsc::Sender<T>,
    r: mpsc::Receiver<T>,
}

unsafe impl<T> Send for ChannelPair<T> {}
unsafe impl<T> Sync for ChannelPair<T> {}

impl<T> ChannelPair<T> {
    fn new() -> ChannelPair<T> {
        let (s, r) = mpsc::channel();

        ChannelPair { s, r }
    }
}

pub struct Executor {
    // Task queue
    storage: Mutex<HashMap<u64, Task>>,

    // Channels for the main thread.
    chan: ChannelPair<Note>,

    // Storage for the main task. (spawned by Executor::start)
    main: Mutex<Option<Task>>,

    // Channels for other threads.
    o_chan: ChannelPair<Note>,

    // task ids
    count: AtomicU64,
}

impl Executor {
    fn new() -> Executor {
        let chan = ChannelPair::new();
        let o_chan = ChannelPair::new();

        Executor {
            storage: Mutex::new(HashMap::with_capacity(4096)),
            main: Mutex::new(None),
            chan,
            o_chan,
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
        let (task, note, handle) = Task::new(f, num, exec.chan.s.clone());
        drop(handle);

        *exec.main.lock().unwrap() = Some(task);
        Executor::get().chan.s.send(note).unwrap();

        let thread_handle = thread::spawn(move || {
            while let Ok(n) = Executor::get().o_chan.r.recv() {
                if n.0 == u64::MAX {
                    break;
                }

                let mut storage = Executor::get().storage.lock().unwrap();

                let task = storage.remove(&n.0).expect("no task with that id");
                let state = task.poll();

                if !state {
                    println!("Task not ready!");
                    let mut st = Executor::get().storage.lock().unwrap();
                    st.insert(n.0, task);
                }
            }
        });

        let mut done = false;
        while !done {
            dbg!(done);
            while let Ok(_n) = Executor::get().chan.r.recv() {
                let exec = Executor::get();

                let task = exec.main.lock().unwrap();

                let state = task.as_ref().unwrap().poll();
                dbg!(state);

                done = state;
                if state {
                    exec.o_chan.s.send(Note(u64::MAX)).unwrap();
                    break;
                }
            }
        }

        let _ = thread_handle.join();
    }
    pub fn spawn<F>(f: F) -> TaskHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let exec = Executor::get();

        let num = exec.count.fetch_add(1, Ordering::SeqCst);
        let (task, note, handle) = Task::new(f, num, exec.o_chan.s.clone());
        Executor::add_task(task, note.0);

        println!("runtime: registered task with id: {}", &note.0);
        exec.o_chan.s.send(note).unwrap();
        handle
    }
}

// loop {
//     while let Ok(n) = Executor::get().recv.inner.recv() {
//         println!("runtime: polling task with id: {0}", n.0);

//         drop(storage);
//     }
