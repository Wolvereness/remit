use core::{
    mem::ManuallyDrop,
    ptr::write,
};

use super::super::{
    Values,
    RemitBack,
};

#[cfg(feature = "alloc")]
use super::super::References;

impl<O> RemitBack<'_, O> {
    pub(crate) fn impl_provide(self, value: O) {
        let this = ManuallyDrop::new(self);
        unsafe {
            // SOUND: wont call check again due to skipping drop via ManuallyDrop
            if this.check() {
                // SOUND: check returned true
                this.write(value);
            }
        }
    }

    /// Must only be called once.
    unsafe fn check(&self) -> bool {
        // SOUND: only called once, see indirection
        (self.indirection)(self)
    }

    /// May only be called after check returns true.
    unsafe fn write(&self, value: O) {
        // SOUND: check() insured that RemitFuture hadn't been dropped
        write(self.data, Some(value))
    }

    fn remove<T>(&self, values: &mut Values<T, O>) -> bool {
        values.remove(self.data)
    }

    pub(crate) unsafe fn indirection_stack<T>(&self) -> bool {
        let values = &mut *(self.indirection_ctx as *mut Values<T, O>);
        self.remove(values)
    }

    #[cfg(feature = "alloc")]
    pub(crate) unsafe fn indirection_boxed<T>(&self) -> bool {
        let references: *const References<T, O> = self.indirection_ctx as _;
        if !References::strong(references) {
            References::dropping(references);
            return false;
        }
        // SOUND: strong reference exists
        References::dropping(references);
        self.remove((&*references).values())
    }
}

impl<O> Drop for RemitBack<'_, O> {
    fn drop(&mut self) {
        // SOUND: `drop` only called once,
        // and other calls use ManuallyDrop.
        let _ = unsafe { self.check() };
    }
}
