use crate::reactor::{Handle, Reactor};
use crate::task::handle::TaskHandle;
use crate::task::note::Note;
use crate::task::task::Task;
use log::{error, info};
use slab::Slab;
use std::cell::{Cell, UnsafeCell};
use std::mem::MaybeUninit;
use std::panic;
use std::ptr::addr_of_mut;
use std::sync::{Arc, OnceLock, RwLock, Weak, atomic::AtomicBool, atomic::Ordering, mpsc};
use std::thread::JoinHandle;
use std::thread_local;

use super::threads::ThreadPool;

use super::cx_box::CxBox;

thread_local! {
    static EXEC: CxBox<Weak<ExecutorHandle>> = CxBox::new();
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
    pool: UnsafeCell<ThreadPool<Note>>,

    // I/O Reactor
    reactor: Reactor,
}

unsafe impl Sync for ExecutorHandle {}
unsafe impl Send for ExecutorHandle {}

impl ExecutorHandle {
    pub(crate) fn reactor_fn<F, T>(self: Arc<Self>, f: F) -> T
    where
        F: FnOnce(&Reactor) -> T,
    {
        f(&self.reactor)
    }

    pub(crate) fn pool_fn<F, T>(self: &Arc<Self>, function: F) -> T
    where
        F: FnOnce(&ThreadPool<Note>) -> T,
    {
        function(unsafe { &*self.pool.get() })
    }
}

pub enum RtState {
    Good,
    MainTaskPanicked,
    Abnormal,
}

pub struct Executor {
    /// Handle to the Executor,
    handle: Arc<ExecutorHandle>,

    /// JoinHandle for the Reactor's thread.
    reactor_handle: MaybeUninit<JoinHandle<()>>,
}

impl Executor {
    fn new_base(amnt: usize) -> Executor {
        let chan = ChannelPair::new();
        let (reactor, handle) = Reactor::new().expect("failed to create reactor");

        let handle = Arc::new(ExecutorHandle {
            storage: RwLock::new(Slab::with_capacity(4096)),
            chan,
            handle,
            pool: UnsafeCell::new(ThreadPool::new(amnt)),
            reactor,
        });

        Executor {
            handle,
            reactor_handle: MaybeUninit::uninit(),
        }
    }

    pub fn new(amnt: usize) -> Executor {
        let mut runtime = Executor::new_base(amnt);
        EXEC.with(|cell| {
            cell.set(Arc::downgrade(&runtime.handle))
                .expect("failed setting global handle")
        });

        unsafe {
            let amnt = (*runtime.handle.pool.get()).amount;
            (*runtime.handle.pool.get())
                .start(amnt, thread_function)
                .expect("failed to start thread pool");
        };

        let handle = runtime
            .handle
            .reactor
            .start()
            .expect("failed to start reactor");

        let ptr = runtime.reactor_handle.as_mut_ptr();
        unsafe { ptr.write(handle) };

        runtime
    }

    #[inline]
    pub fn get() -> Arc<ExecutorHandle> {
        EXEC.with(
            |cell| match cell.get_ref().expect("runtime is not running").upgrade() {
                None => panic!("runtime is gone"),
                Some(handle) => handle,
            },
        )
    }

    pub fn block_on<F>(&mut self, f: F) -> Result<(), RtState>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let exec = Executor::get();

        let task = Task::new_alone(f, u64::MAX - 1, exec.chan.s.clone());

        let mut state = RtState::Good;

        loop {
            let out = panic::catch_unwind(|| task.poll());

            let ready = match out {
                // As the `Err` can contain the panic payload
                // we ignore it.
                // with debug assertions, we will attempt to `dbg!` it.
                Err(_err) => {
                    error!("main task panicked");
                    state = RtState::MainTaskPanicked;

                    #[cfg(debug_assertions)]
                    let _ = dbg!(_err.downcast::<String>());

                    break;
                }

                Ok(ready) => ready,
            };

            if ready {
                break;
            };

            'inner: loop {
                let n = exec.chan.r.recv().expect("receiver failed at block_on");

                if n.0 != u64::MAX - 1 {
                    self.handle.pool_fn(|pool| {
                        let _ = pool.deploy(n);
                    });
                } else {
                    break 'inner;
                }

                info!("woke up thread, notif id: {}", n.0);
            }
        }

        match state {
            RtState::Good => Ok(()),
            RtState::MainTaskPanicked => Err(RtState::MainTaskPanicked),
            RtState::Abnormal => Err(RtState::Abnormal),
        }
    }

    pub fn shutdown(mut self) {
        let exec = Executor::get();
        let _ = exec.handle.shutdown();
        exec.pool_fn(|pool| {
            let _ = pool.broadcast(Note(u64::MAX));
            let _ = pool.join();
        });

        // Safety:
        //
        // Nobody has the handle as we have consumed the runtime.
        unsafe { self.reactor_handle.assume_init_drop() };

        // Safety:
        //
        // Everything was shutdown, no other thread might accidentally execute this.
        unsafe { EXEC.with(|c| c.clean()) };
    }

    /// Spawn a future onto the Runtime.
    #[inline]
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

        // If it fails, something terrible happened.
        exec.pool_fn(|pool| pool.deploy(note))
            .expect("failed to send note to runtime");

        handle
    }
}

impl std::ops::Drop for Executor {
    fn drop(&mut self) {}
}

fn thread_function(
    rt_weak: Weak<ExecutorHandle>,
    r: mpsc::Receiver<Note>,
    boolean: Arc<AtomicBool>,
) {
    while let Ok(n) = r.recv() {
        boolean.store(true, Ordering::SeqCst);

        let rt = match rt_weak.upgrade() {
            // The runtime has been dropped.
            None => break,

            Some(rt) => rt,
        };

        if n.0 == u64::MAX {
            break;
        }

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
