use crate::reactor::{Handle, Reactor};
use crate::task::handle::TaskHandle;
use crate::task::note::Note;
use crate::task::task::Task;
use log::info;
use slab::Slab;
use std::sync::{Arc, OnceLock, RwLock, Weak, atomic::AtomicBool, atomic::Ordering, mpsc};
use std::thread_local;

use super::threads::ThreadPool;

thread_local! {
    static EXEC: OnceLock<Weak<ExecutorHandle>> = OnceLock::new();
}

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

pub struct ExecutorHandle {
    // Task queue
    storage: RwLock<Slab<Task>>,

    // Channels for the main thread.
    chan: ChannelPair<Note>,

    // Might be useful later
    // Handle to reactor
    #[allow(dead_code)]
    handle: Arc<Handle>,

    // Thread pool
    pool: ThreadPool<Note>,

    // I/O Reactor
    reactor: Reactor,
}

impl ExecutorHandle {
    pub(crate) fn reactor_fn<F, T>(self: &Arc<Self>, f: F) -> T
    where
        F: FnOnce(&Reactor) -> T,
    {
        f(&self.reactor)
    }
}

pub struct Executor {
    /// Handle to the Executor,
    handle: Arc<ExecutorHandle>,
}

impl Executor {
    fn new_base(amnt: usize) -> Executor {
        let chan = ChannelPair::new();
        let (reactor, handle) = Reactor::new();

        fn func(r: mpsc::Receiver<Note>, boolean: Arc<AtomicBool>) {
            while let Ok(n) = r.recv() {
                boolean.store(true, Ordering::SeqCst);

                if n.0 == u64::MAX {
                    break;
                }

                let rt = Executor::get();

                let storage = rt.storage.read().unwrap();
                let task = storage.get(n.0 as usize).unwrap();
                let ready = task.poll();
                drop(storage);

                if ready {
                    let mut st = rt.storage.write().unwrap();
                    let _ = st.remove(n.0 as usize);
                    info!("removed task (id: {})", n.0);
                }

                boolean.store(false, Ordering::SeqCst)
            }
        }

        let handle = Arc::new(ExecutorHandle {
            storage: RwLock::new(Slab::with_capacity(4096)),
            chan,
            handle,
            pool: ThreadPool::new(amnt, func).unwrap(),
            reactor,
        });

        Executor { handle }
    }

    pub fn new(amnt: usize) -> Executor {
        let runtime = Executor::new_base(amnt);
        EXEC.with(|cell| {
            cell.set(Arc::downgrade(&runtime.handle))
                .expect("failed setting global handle");
        });

        runtime
    }

    pub fn get() -> Arc<ExecutorHandle> {
        EXEC.with(
            |cell| match cell.get().expect("runtime is not running").upgrade() {
                None => panic!("runtime is gone"),
                Some(handle) => handle,
            },
        )
    }

    pub fn block_on<F>(&mut self, f: F)
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
                exec.pool.broadcast(Note(u64::MAX));
                let _ = exec.pool.join();
                drop(task);

                break;
            } else {
                let _ = exec.chan.r.recv();
            }
        }
    }

    /// Spawn a future onto the Runtime.
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

        // If it fails, something terrible happened.
        exec.pool
            .deploy(note)
            .expect("failed to send note to runtime");

        handle
    }
}

impl std::ops::Drop for Executor {
    fn drop(&mut self) {}
}
