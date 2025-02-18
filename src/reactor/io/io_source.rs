use crate::reactor::reactor::Direction;
use log::debug;
use mio::Token;
use mio::event::Event;
use std::task::Waker;

const WAKER_AMNT: usize = 64;

/// Contains wakers.
pub struct WakerBox {
    readers: Vec<Waker>,
    writers: Vec<Waker>,
    index: usize,
}

impl WakerBox {
    pub(crate) fn put(&mut self, waker: &Waker, dir: Direction) {
        let target = match dir {
            Direction::Read => &mut self.readers,
            Direction::Write => &mut self.writers,
        };

        match target.get(self.index) {
            None => target.push(waker.clone()),
            Some(orig_waker) => {
                if !orig_waker.will_wake(waker) {
                    target[self.index] = waker.clone();
                }
            }
        }

        self.index += 1;
    }

    pub(crate) fn wake_all(&mut self, dir: Direction) {
        let target = match dir {
            Direction::Read => &mut self.readers,
            Direction::Write => &mut self.writers,
        };

        target.drain(..).for_each(|waker| waker.wake_by_ref());
    }

    pub(crate) fn wake_all_no_dir(&mut self) {
        let readers = self.readers.drain(..);
        let writers = self.writers.drain(..);

        readers.chain(writers).for_each(|waker| waker.wake_by_ref());
    }

    fn has_wakers(&self) -> bool {
        self.readers.len() > 0 || self.writers.len() > 0
    }
}

/// Represents a connection between a waker and the reactor
pub struct IoSource {
    wakers: WakerBox,
    token: Token,
}

impl IoSource {
    pub fn new(token: usize) -> IoSource {
        IoSource {
            wakers: WakerBox {
                readers: Vec::with_capacity(WAKER_AMNT),
                writers: Vec::with_capacity(WAKER_AMNT),
                index: 0,
            },
            token: Token(token),
        }
    }

    pub fn has_wakers(&self) -> bool {
        self.wakers.has_wakers()
    }

    pub fn wake_with_event(&mut self, ev: &Event) {
        match (ev.is_readable(), ev.is_writable()) {
            (true, false) => self.wakers.wake_all(Direction::Read),
            (false, true) => self.wakers.wake_all(Direction::Write),
            (true, true) => self.wakers.wake_all_no_dir(),

            (..) => debug!("non readable and non writable event"),
        }
        // todo: handle closing and stuff
    }

    pub(crate) fn put(&mut self, waker: &Waker, dir: Direction) {
        self.wakers.put(waker, dir)
    }
}
