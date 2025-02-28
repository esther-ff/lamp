use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ops::Drop;
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) struct CxBox<T> {
    data: UnsafeCell<MaybeUninit<T>>,
    bool: AtomicBool,
}

unsafe impl<T> Sync for CxBox<T> {}
unsafe impl<T> Send for CxBox<T> {}

impl<T> CxBox<T> {
    /// Creates a new `Cxbox`
    pub(crate) const fn new() -> Self {
        CxBox {
            data: UnsafeCell::new(MaybeUninit::uninit()),
            bool: AtomicBool::new(false),
        }
    }

    /// Sets the value if it's not initialised
    pub(crate) fn set(&self, val: T) -> Result<(), ()> {
        dbg!(self.bool.load(Ordering::SeqCst));
        if self.bool.load(Ordering::SeqCst) {
            return Err(());
        };

        unsafe {
            (*self.data.get()).write(val);
        };

        self.bool.store(true, Ordering::SeqCst);

        Ok(())
    }

    /// Obtains an immutable reference to the data inside.
    pub(crate) fn get_ref(&self) -> Option<&T> {
        if self.bool.load(Ordering::SeqCst) {
            // Safety:
            //
            // Checking the `bool` inside the `CxBox` makes sure we only access a reference
            // to initialized data.
            unsafe {
                let ptr = self.data.get();

                Some((*ptr).assume_init_ref())
            }
        } else {
            None
        }
    }

    /// This is only safe while calling from one thread.
    /// this performs an un-atomic operation on the data inside
    /// Partial safety is provided by first setting the bool inside to false
    /// preventing any more reads.
    pub(crate) unsafe fn clean(&self) {
        if !self.bool.load(Ordering::SeqCst) {
            return;
        }

        self.bool.store(false, Ordering::SeqCst);

        // Safety:
        //
        // The bool is first changed to `false` so
        // no invocation during the dropping of the value
        // accesses uninitialised data.
        unsafe {
            let ptr = self.data.get();

            (*ptr).assume_init_drop();
        }
    }
}

impl<T> Drop for CxBox<T> {
    fn drop(&mut self) {
        unsafe { self.clean() }
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::CxBox;

    #[test]
    fn create_cx_box() {
        let cx_box = CxBox::new();

        assert!(
            cx_box.set("Dance the Carmagnole!").is_ok(),
            "failed to set the value"
        );

        unsafe {
            cx_box.clean();
        }

        assert!(
            cx_box.get_ref().is_none(),
            "incorrect handling of uninit data"
        );

        drop(cx_box);
    }

    #[test]
    fn create_global_cx_box() {
        static CX_BOX: CxBox<usize> = CxBox::new();
        let r1 = CX_BOX.set(125);

        assert!(r1.is_ok(), "failed to set value");

        let thread_one = thread::spawn(|| CX_BOX.get_ref());
        assert!(CX_BOX.get_ref().unwrap() == &125, "not equal");
        assert!(thread_one.join().is_ok(), "thread failed");

        unsafe {
            CX_BOX.clean();
        };

        assert!(CX_BOX.set(126).is_ok(), "set failed");
        assert!(
            CX_BOX.get_ref().is_some_and(|val| val == &126),
            "invalid or missing value"
        );
    }
}
