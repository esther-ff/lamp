use crate::task::handle::TaskHandle;
use crate::task::note::Note;
use crate::task::task::Task;
use log::{debug, info};
use slab::Slab;
use std::sync::atomic::AtomicU64;

use std::sync::{mpsc, Mutex, OnceLock, RwLock};
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
    storage: RwLock<Slab<Task>>,

    // Channels for the main thread.
    chan: ChannelPair<Note>,

    // Storage for the main task. (spawned by Executor::start)
    main: Mutex<Option<Task>>,

    // Channels for other threads.
    o_chan: ChannelPair<Note>,
}

impl Executor {
    fn new() -> Executor {
        let chan = ChannelPair::new();
        let o_chan = ChannelPair::new();

        Executor {
            storage: RwLock::new(Slab::with_capacity(4096)),
            main: Mutex::new(None),
            chan,
            o_chan,
        }
    }

    pub fn build() {
        EXEC.get_or_init(Executor::new);
    }

    pub fn get() -> &'static Executor {
        EXEC.get().expect("runtime is not running")
    }

    pub fn start<F>(f: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static + std::fmt::Debug,
    {
        let exec = Executor::get();
        let (task, _, _) = Task::new(f, u64::MAX - 1, exec.chan.s.clone());

        let thread_handle = thread::spawn(move || {
            while let Ok(n) = Executor::get().o_chan.r.recv() {
                if n.0 == u64::MAX {
                    break;
                }

                let storage = Executor::get().storage.read().unwrap();

                let task = storage.get(n.0 as usize).unwrap();
                let ready = task.poll();
                dbg!(ready);
                drop(storage);

                if ready {
                    let mut st = Executor::get().storage.write().unwrap();
                    let _ = st.remove(n.0 as usize);
                    info!("removed task (id: {})", n.0);
                }
            }
        });

        loop {
            let ready = task.poll();

            if ready {
                exec.o_chan.s.send(Note(u64::MAX)).unwrap();
                let _ = thread_handle.join();
                drop(task);
                break;
            } else {
                let _ = exec.chan.r.recv();
            }
        }
    }
    pub fn spawn<F>(f: F) -> TaskHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let exec = Executor::get();

        let mut storage = exec.storage.write().unwrap();
        let num = storage.vacant_key();

        let (task, note, handle) = Task::new(f, num as u64, exec.chan.s.clone());
        storage.insert(task);
        drop(storage);

        info!("registered task with id: {}", &note.0);
        exec.o_chan.s.send(note).unwrap();
        handle
    }
}

// loop {
//     while let Ok(n) = Executor::get().recv.inner.recv() {
//         println!("runtime: polling task with id: {0}", n.0);

//         drop(storage);
//     }
