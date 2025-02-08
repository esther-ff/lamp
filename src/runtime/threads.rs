use std::sync::atomic::AtomicBool;
use std::sync::{Arc, atomic, mpsc};
use std::thread::{self, available_parallelism};

use slab::Slab;

pub(crate) struct WorkerThread<T> {
    name: String,
    occupied: Arc<AtomicBool>,
    tasks_done: u64,
    sender: mpsc::Sender<T>,
    handle: Option<thread::JoinHandle<()>>,
    backup_fn: fn(mpsc::Receiver<T>, Arc<AtomicBool>),
    ok: bool,
}

impl<T: Send + 'static> WorkerThread<T> {
    pub(crate) fn new(name: String, f: fn(mpsc::Receiver<T>, Arc<AtomicBool>)) -> Self {
        let (sender, receiver) = mpsc::channel();
        let arc_bool = Arc::new(AtomicBool::new(false));

        let clone = Arc::clone(&arc_bool);
        let mut worker = Self {
            name,
            occupied: arc_bool,
            tasks_done: 0,
            sender,
            handle: None,
            backup_fn: f,
            ok: true,
        };

        let handle = thread::spawn(move || f(receiver, clone));
        worker.handle = Some(handle);
        worker
    }

    pub(crate) fn push(&mut self, notif: T) -> Result<(), mpsc::SendError<T>> {
        self.tasks_done += 1;
        self.sender.send(notif)
    }

    pub(crate) fn rebuild(&mut self) {
        let (sender, receiver) = mpsc::channel();
        let clone = Arc::clone(&self.occupied);
        let func = self.backup_fn.clone();

        self.sender = sender;
        self.tasks_done = 0;
        self.occupied.swap(false, atomic::Ordering::SeqCst);

        let handle = Some(thread::spawn(move || func(receiver, clone)));
        self.handle = handle;
    }
}

pub(crate) struct ThreadPool<Notif> {
    workers: Slab<WorkerThread<Notif>>,
    occupied: Slab<WorkerThread<Notif>>,
}

impl<Notif: Send + 'static> ThreadPool<Notif> {
    pub(crate) fn new(amnt: usize, f: fn(mpsc::Receiver<Notif>, Arc<AtomicBool>)) -> Self {
        let cores = available_parallelism().expect("failed to check cpu thread count");
        let not_overflow = amnt <= cores.into();
        assert!(not_overflow);

        let mut slab: Slab<WorkerThread<Notif>> = Slab::with_capacity(amnt);
        slab.iter_mut()
            .for_each(|(key, value)| *value = WorkerThread::new(format!("thread-{key}"), f));

        Self {
            workers: slab,
            occupied: Slab::new(),
        }
    }

    pub(crate) fn deploy(&mut self, n: Notif) -> Result<(), mpsc::SendError<Notif>> {
        let mut chosen = 0;
        let mut count = 0;

        self.workers
            .iter_mut()
            .filter(|(_, v)| !v.ok)
            .for_each(|(_, v)| {
                v.rebuild();
            });

        self.workers
            .iter_mut()
            .filter(|(_k, v)| !v.occupied.load(atomic::Ordering::Relaxed))
            .for_each(|(k, v)| match count > v.tasks_done {
                true => {
                    count = v.tasks_done;
                    chosen = k;
                }

                false => (),
            });

        self.workers
            .get_mut(chosen)
            .expect("expected a worker here")
            .push(n)
    }
}
