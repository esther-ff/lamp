// Notification to the Runtime.
#[derive(Clone, Copy)]
pub struct Note(pub u64);

unsafe impl Send for Note {}
