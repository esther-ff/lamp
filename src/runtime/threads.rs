use log::{error, info, warn};
use slab::Slab;
use std::cell::Cell;
use std::fmt::{self, Debug, Formatter};
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, atomic, atomic::Ordering, mpsc};
use std::thread::{self, available_parallelism};

pub(crate) struct WorkerThread<T: Send + 'static> {
    // Is it currently occupied
    occupied: Arc<AtomicBool>,

    // How much work has been done
    tasks_received: AtomicU64,

    // Channel Pair
    sender: mpsc::Sender<T>,
    receiver: Option<mpsc::Receiver<T>>,

    // Task handle
    handle: Cell<Option<thread::JoinHandle<()>>>,

    // Backup of the function used to run the thread.
    func: fn(mpsc::Receiver<T>, Arc<AtomicBool>),

    // Is the thread okay?
    ok: bool,
}

impl<T: Send + 'static> Debug for WorkerThread<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkerThread")
            .field("occupied", &self.occupied)
            .field("tasks_received", &self.tasks_received)
            .field("sender", &self.sender)
            .field("receiver", &self.receiver)
            .field("func", &self.func)
            .field("ok", &self.ok)
            .finish_non_exhaustive()
    }
}

impl<T: Send + 'static> WorkerThread<T> {
    pub(crate) fn new(func: fn(mpsc::Receiver<T>, Arc<AtomicBool>)) -> Self {
        let (sender, receiver) = mpsc::channel();
        let arc_bool = Arc::new(AtomicBool::new(false));

        Self {
            occupied: arc_bool,
            tasks_received: AtomicU64::new(0),
            sender,
            receiver: Some(receiver),
            handle: Cell::new(None),
            func,
            ok: true,
        }
    }

    pub(crate) fn start(&mut self) -> io::Result<()> {
        let clone = Arc::clone(&self.occupied);
        let func = self.func.clone();
        let receiver = self.receiver.take().unwrap();

        let handle = thread::Builder::new().spawn(move || func(receiver, clone))?;

        self.handle.set(Some(handle));
        Ok(())
    }

    pub(crate) fn push(&self, notif: T) -> Result<(), mpsc::SendError<T>> {
        self.tasks_received.fetch_add(1, Ordering::Relaxed);
        self.sender.send(notif)
    }

    pub(crate) fn join(&self) -> thread::Result<()> {
        let handle = self.handle.take();
        let mut result = Ok(());

        if handle.is_some() {
            result = handle.unwrap().join();
        }

        result
    }

    /// Rebuilds the thread.
    // TODO: make it use `&self`
    pub(crate) fn rebuild(&mut self) {
        let (sender, receiver) = mpsc::channel();
        let clone = Arc::clone(&self.occupied);
        let func = self.func.clone();

        self.sender = sender;
        self.tasks_received = AtomicU64::new(0);
        self.occupied.swap(false, atomic::Ordering::SeqCst);

        let handle = Some(thread::spawn(move || func(receiver, clone)));
        self.handle.set(handle);
    }
}

pub(crate) struct ThreadPool<Notif: Send + 'static> {
    workers: Slab<WorkerThread<Notif>>,
    occupied: Slab<WorkerThread<Notif>>,
}

impl<Notif: Send + 'static + Copy> ThreadPool<Notif> {
    /// Broadcasts a message to each worker.
    /// Copies the argument.
    pub(crate) fn broadcast(&self, notif: Notif) {
        self.workers.iter().for_each(|(_, thread)| {
            thread.push(notif);
        })
    }
}

impl<Notif: Send + 'static + Clone> ThreadPool<Notif> {
    /// Broadcasts a message to each worker.
    /// Clones the argument.
    pub(crate) fn broadcast_clone(&self, notif: Notif) {
        self.workers.iter().for_each(|(_, thread)| {
            thread.push(notif.clone());
        })
    }
}

impl<Notif: Send + 'static> ThreadPool<Notif> {
    pub(crate) fn new(
        amnt: usize,
        f: fn(mpsc::Receiver<Notif>, Arc<AtomicBool>),
    ) -> io::Result<Self> {
        let cores = available_parallelism().expect("failed to check cpu thread count");
        let not_overflow = amnt <= cores.into();
        assert!(not_overflow);

        let mut pool = Self {
            workers: Slab::with_capacity(amnt),
            occupied: Slab::new(),
        };

        loop {
            println!("Guh");
            if pool.workers.len() == amnt {
                break;
            }

            info!("creating thread");
            let mut th = WorkerThread::new(f);

            if th.start().is_err() {
                // returns a pool with a reduced size
                // however the state is okay.
                error!("fail during population of the pool, giving back a incomplete pool");
                break;
            }
            pool.workers.insert(th);
        }

        pool.workers.shrink_to_fit();

        Ok(pool)
    }

    /// Deploys a task to a chosen worker.
    pub(crate) fn deploy(&self, n: Notif) -> Result<(), mpsc::SendError<Notif>> {
        let mut chosen = 0;
        let mut count = 0;

        // Reimplement in a lock-free way.
        // self.workers
        //     .iter_mut()
        //     .filter(|(_, v)| !v.ok)
        //     .for_each(|(_, v)| {
        //         v.rebuild();
        //     });

        self.workers
            .iter()
            .filter(|(_k, v)| !v.occupied.load(atomic::Ordering::Relaxed))
            .for_each(|(k, v)| {
                let received = v.tasks_received.load(Ordering::Relaxed);
                match count > received {
                    true => {
                        count = received;
                        chosen = k;
                    }

                    false => (),
                };
            });

        dbg!(&self.workers);
        dbg!(chosen);
        self.workers
            .get(chosen)
            .expect("expected a worker here")
            .push(n)
    }

    // Waits for each worker to finish.
    pub(crate) fn join(&self) -> thread::Result<()> {
        let mut result = Ok(());

        self.workers.iter().for_each(|(_, v)| {
            let res = v.join();
            if res.is_err() {
                result = res
            }
        });

        result
    }
}

impl<T: Send + 'static> std::ops::Drop for ThreadPool<T> {
    fn drop(&mut self) {
        self.workers.iter_mut().for_each(|(_, v)| {
            // Result is ignored because we are dropping the pool.
            let _ = v.join();
        });
    }
}

unsafe impl<T: Send> Send for ThreadPool<T> {}
unsafe impl<T: Send> Sync for ThreadPool<T> {}
