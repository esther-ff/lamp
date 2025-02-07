// Notification to the Runtime.

use std::cmp::{Eq, Ord};

#[derive(Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Debug)]
pub struct Note(pub u64);

unsafe impl Send for Note {}
