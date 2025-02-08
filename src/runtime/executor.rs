use crate::reactor::{Handle, Reactor};
use crate::task::handle::TaskHandle;
use crate::task::note::Note;
use crate::task::task::Task;
use log::info;
use slab::Slab;
use std::sync::{Arc, Mutex, OnceLock, RwLock, atomic::AtomicBool, atomic::Ordering, mpsc};

use super::threads::ThreadPool;

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

    // Channels for other threads.
    o_chan: ChannelPair<Note>,

    // I/O Reactor
    reactor: Reactor,

    // Might be useful later
    // Handle to reactor
    #[allow(dead_code)]
    handle: Arc<Handle>,

    // Thread pool
    pool: Mutex<ThreadPool<Note>>,
}

impl Executor {
    fn new(amnt: usize) -> Executor {
        let chan = ChannelPair::new();
        let o_chan = ChannelPair::new();
        let (reactor, handle) = Reactor::new();

        fn func(r: mpsc::Receiver<Note>, boolean: Arc<AtomicBool>) {
            while let Ok(n) = r.recv() {
                boolean.store(true, Ordering::SeqCst);

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

                boolean.store(false, Ordering::SeqCst)
            }
        }
        Executor {
            storage: RwLock::new(Slab::with_capacity(4096)),
            chan,
            o_chan,
            reactor,
            handle,
            pool: Mutex::new(ThreadPool::new(amnt, func)),
        }
    }

    pub fn build(amnt: usize) {
        EXEC.get_or_init(|| Executor::new(amnt));
    }

    pub fn get() -> &'static Executor {
        EXEC.get().expect("runtime is not running")
    }

    pub fn get_reactor() -> &'static Reactor {
        &Executor::get().reactor
    }

    pub fn start<F>(f: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let exec = Executor::get();
        exec.reactor.start();
        let (task, _, _) = Task::new(f, u64::MAX - 1, exec.chan.s.clone());

        loop {
            let ready = task.poll();

            if ready {
                exec.o_chan.s.send(Note(u64::MAX)).unwrap();
                let _ = exec.pool.lock().unwrap().join();
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
        // Handle properly.
        let _ = exec.pool.lock().unwrap().deploy(note);
        handle
    }
}
