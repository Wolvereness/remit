use core::{
    mem::ManuallyDrop,
    ptr::write,
};

#[cfg(feature = "alloc")]
use alloc::rc::Rc;

use super::super::{
    Values,
    RemitBack,
    Indirection,
};

#[cfg(feature = "alloc")]
use super::{
    cycler::Cycler,
    super::References,
};

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
        // SOUND: only called once, see indirection_{stack|boxed}
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

    pub(crate) fn indirection_stack_ptr<'s, T>(ptr: *mut Values<T, O>) -> (Indirection<'s, O>, *const ()) {
        (
            RemitBack::<'s, O>::indirection_stack::<T>,
            ptr as _,
        )
    }

    /// May only be called as-constructed by indirection_stack_ptr,
    /// and only once.
    // NEED: erasing <T>
    unsafe fn indirection_stack<T>(&self) -> bool {
        let values = &mut *(self.indirection_ctx as *mut Values<T, O>);
        self.remove(values)
    }

    #[cfg(feature = "alloc")]
    /// May only be called from the boxed variant.
    pub(crate) unsafe fn indirection_boxed_ptr<'s, 'a, T, P>(
        ptr: *const References<T, O>,
        rc: &'a Option<Rc<Cycler<P, T, O>>>,
    ) -> (Indirection<'s, O>, *const ()) {
        // SOUND: boxed variant always has the RC
        let _ = Rc::downgrade(unsafe { rc.as_ref().unwrap_unchecked() }).into_raw();
        (
            RemitBack::<'s, O>::indirection_boxed::<T>,
            ptr as _,
        )
    }

    #[cfg(feature = "alloc")]
    /// May only be called as-constructed by indirection_boxed_ptr,
    /// and only once.
    // NEED: erasing <T>
    unsafe fn indirection_boxed<T>(&self) -> bool {
        let references: *const References<T, O> = self.indirection_ctx as _;
        let strong = References::strong(references);
        // SOUND: indirection_boxed_ptr increased the weak count
        // SOUND: only called once
        References::dropping(references);
        if !strong {
            return false;
        }
        // SOUND: strong reference exists
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
