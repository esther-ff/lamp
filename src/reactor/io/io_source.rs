use crate::reactor::reactor::Direction;
use log::debug;
use mio::Token;
use mio::event::Event;
use std::panic;
use std::task::Waker;

use log::error;
use std::mem::MaybeUninit;

const WAKER_AMNT: usize = 64;
struct WakerList {
    blk: [MaybeUninit<Waker>; WAKER_AMNT],
    cursor: usize,
}

impl WakerList {
    fn new() -> Self {
        WakerList {
            blk: [const { MaybeUninit::uninit() }; WAKER_AMNT],
            cursor: 0,
        }
    }

    fn wake_all(&mut self) {
        while self.cursor != 0 {
            let uninit = &mut self.blk[self.cursor];

            unsafe {
                let waker = uninit.assume_init_mut();
                let res = panic::catch_unwind(|| waker.wake_by_ref());
                if res.is_err() {
                    error!("panic while waking up waker.")
                }
                uninit.assume_init_drop();
            };

            self.cursor -= 1;
        }
    }

    fn put(&mut self, waker: Waker) {
        self.cursor += 1;
        self.blk[self.cursor].write(waker);
    }

    fn full(&self) -> bool {
        self.cursor == WAKER_AMNT
    }

    fn elements(&self) -> usize {
        self.cursor
    }
}

/// Contains wakers.
pub struct Wakers {
    rd: WakerList,
    wr: WakerList,
}

impl Wakers {
    pub(crate) fn put(&mut self, waker: &Waker, dir: Direction) {
        let list = match dir {
            Direction::Read => &mut self.rd,
            Direction::Write => &mut self.wr,
        };

        list.put(waker.clone())
    }

    pub(crate) fn wake_all(&mut self, dir: Direction) {
        if !self.has_wakers() {
            return;
        };

        let target = match dir {
            Direction::Read => &mut self.rd,
            Direction::Write => &mut self.wr,
        };

        debug!(
            "waking {} wakers for direction: {:#?}",
            target.elements(),
            dir
        );
        target.wake_all();
    }

    pub(crate) fn wake_all_no_dir(&mut self) {
        if !self.has_wakers() {
            return;
        };

        debug!(
            "waking up all {} wakers",
            self.rd.elements() + self.wr.elements()
        );

        self.rd.wake_all();
        self.wr.wake_all();
    }

    fn has_wakers(&self) -> bool {
        self.rd.elements() > 0 || self.wr.elements() > 0
    }
}

/// Represents a connection between a waker and the reactor
pub struct IoSource {
    wakers: Wakers,
    token: Token,
}

impl IoSource {
    pub fn new(token: usize) -> IoSource {
        IoSource {
            wakers: Wakers {
                rd: WakerList::new(),
                wr: WakerList::new(),
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
