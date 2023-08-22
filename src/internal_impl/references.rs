use core::{
    cell::{
        Cell,
        UnsafeCell,
    },
    ptr::{
        addr_of,
        null_mut,
    }
};

use super::super::{
    Values,
    Cycler,
};

pub struct References<T, O> {
    pub(crate) interchange: UnsafeCell<Values<T, O>>,
    dropper: unsafe fn(*mut ()),
    checker: unsafe fn(*mut ()) -> bool,
    pub ptr: Cell<*mut ()>,
}

impl<T, O> References<T, O> {
    pub fn new<P>() -> Self {
        References {
            interchange: UnsafeCell::new(Values::Missing),
            dropper: Cycler::<P, T, O>::do_inner_drop,
            checker: Cycler::<P, T, O>::is_strong,
            // Note that `null_mut` is only until the surrounding Rc gets created.
            ptr: Cell::new(null_mut()),
        }
    }

    /// Must not be have multiple aliases.
    pub unsafe fn values(&self) -> &mut Values<T, O> {
        unsafe { &mut *self.interchange.get() }
    }

    pub unsafe fn strong(this: *const Self) -> bool {
        let inner_ptr = (*addr_of!((*this).ptr)).get();
        // SOUND: checker is not pub, nor was inner_ptr,
        // thus still valid from instantiation
        //
        // SOUND: unsafe-fn, see Cycler::is_strong
        (*addr_of!((*this).checker))(inner_ptr)
    }

    pub unsafe fn dropping(this: *const Self) {
        let inner_ptr = (*addr_of!((*this).ptr)).get();
        // SOUND: dropper is not pub, nor was inner_ptr,
        // thus still valid from instantiation
        //
        // SOUND: dropper only called once for inner_ptr,
        // as inner_ptr only exists in this struct,
        // and dropping is only called once.
        //
        // SOUND: unsafe-fn, see Cycler::do_inner_drop
        (*addr_of!((*this).dropper))(inner_ptr)
    }
}
