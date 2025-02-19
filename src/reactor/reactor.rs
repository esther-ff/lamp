use crate::io::IoSource;

use mio::event::Source;
use mio::{Events, Interest, Poll, Registry, Token};

use std::io::Result as IoResult;
use std::sync::{Arc, Mutex};
use std::task::Context;
use std::thread;
use std::fmt;

use slab::Slab;

use log::debug;

const SHUTDOWN: Token = Token(usize::MAX);
/// represents the interest of the underlying io.
pub enum Direction {
    Read,
    Write,
}

impl fmt::Debug for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Direction::Read => write!(f, "Direction::Read"),
            Direction::Write => write!(f, "Direction::Write")
        }
    }
}

/// Represents the global I/O Reactor.
///
/// Only one exists at anytime.
pub struct Reactor {
    /// Re-usable event pool.
    events: Arc<Mutex<Events>>,

    /// Handle
    handle: Arc<Handle>,

    /// I/O sources
    sources: Arc<Mutex<Slab<IoSource>>>,
}

/// Handle to the I/O Reactor.
pub struct Handle {
    /// Registry belonging to `mio::Poll`
    registry: Registry,

    /// Poll from which we obtain events.
    poll: Mutex<Poll>,

    /// Waker to wake up the thread.
    waker: mio::Waker,
}

impl Handle {
    #[allow(dead_code)]
    pub(crate) fn registry(&self) -> &Registry {
        &self.registry
    }

    pub(crate) fn shutdown(&self) -> IoResult<()> {
        self.waker.wake()
    }

    fn arc_new(registry: Registry, poll: Poll, waker: mio::Waker) -> Arc<Handle> {
        Arc::new(Handle {
            registry,
            poll: Mutex::new(poll),
            waker,
        })
    }
}

impl Reactor {
    /// Create a new Reactor
    pub fn new() -> IoResult<(Reactor, Arc<Handle>)> {
        let poll = Poll::new().expect("failed to create poll");

        let events = Arc::new(Mutex::new(Events::with_capacity(1024)));
        let registry = poll.registry().try_clone().expect("registry clone fail");
        let sources = Arc::new(Mutex::new(Slab::with_capacity(1024)));
        let waker = mio::Waker::new(&registry, SHUTDOWN)?;
        let handle = Handle::arc_new(registry, poll, waker);

        let r = Reactor {
            sources,
            events,
            handle,
        };
        let arc_handle = Arc::clone(&r.handle);
        Ok((r, arc_handle))
    }

    /// Get reference to the Reactor.
    pub fn start(&self) -> IoResult<thread::JoinHandle<()>> {
        // Polling thread
        let arc_events = Arc::clone(&self.events);
        let arc_sources: Arc<Mutex<Slab<IoSource>>> = Arc::clone(&self.sources);
        let handle = Arc::clone(&self.handle);

        let handle = thread::Builder::new()
            .name("IoReactor".to_string())
            .spawn(move || {
                let mut poll = handle.poll.lock().expect("failed loop poll lock");
                let mut events = arc_events.lock().expect("event lock fail");

                loop {
                    debug!("polling for events");
                    
                    match poll.poll(&mut events, None) {
                        Ok(_) => {}
                        Err(e) => panic!("{}", e),
                    }

                    debug!("got io events");

                    for event in events.iter() {
                        debug!("event: {{ token: {}, readable: {}, writable: {}, error: {} }}", 
                        event.token().0, event.is_readable(), event.is_writable(), event.is_error());
                    
                        match event.token() {
                            SHUTDOWN => {
                                debug!("shutting down reactor");
                                return;
                            }

                            _ => {
                                let mut srcs = arc_sources.lock().expect("sources lock in loop failed!");

                                let src = match srcs.get_mut(event.token().0) {
                                    None => panic!(
                                        "Received event for token {}, but no such source is present.",
                                        event.token().0
                                    ),
                                    Some(source) => source,
                                };

                                if src.has_wakers() {
                                    src.wake_with_event(event)
                                };
                            },
                        };
                    }
                }
            })?;

        Ok(handle)
    }

    /// Registers a IO source in the reactor.
    pub fn register(&self, src: &mut impl Source, interest: Interest) -> IoResult<usize> {
        let mut sources = self.sources.lock().expect("failed source lock");
        let token = sources.vacant_key();

        self.handle.registry.register(src, Token(token), interest)?;

        let _ = sources.insert(IoSource::new(token));
        Ok(token)
    }

    /// Reregisters a IO source in the reactor.
    pub fn reregister(&self, src: &mut impl Source, token: usize, intr: Interest) -> IoResult<()> {
        //let sources = Reactor::get().sources.lock().expect("failed sources lock!");
        self.handle.registry.reregister(src, Token(token), intr)
    }

    pub fn attach_waker(&self, cx: &mut Context<'_>, token: Token, dir: Direction) {
        let mut sources = self.sources.lock().expect("failed sources lock!");
        let src = match sources.get_mut(token.0) {
            Some(source) => source,
            None => panic!("Trying to attach waker to an unregistered source!"),
        };

        src.put(cx.waker(), dir);
        drop(sources);
    }
}
