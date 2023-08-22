use core::marker::PhantomData;

#[cfg(feature = "alloc")]
use core::ptr::addr_of;

use crate::Values;

#[cfg(feature = "alloc")]
use crate::References;

pub enum Mode<'a, T, O> {
    Pinned {
        value: *mut Values<T, O>,
        _lifetime: PhantomData<&'a ()>,
    },
    #[cfg(feature = "alloc")]
    Boxed(*const References<T, O>),
}

impl<T, O> Clone for Mode<'_, T, O> {
    fn clone(&self) -> Self {
        *self
    }

    fn clone_from(&mut self, source: &Self) {
        *self = *source
    }
}

impl<T, O> Copy for Mode<'_, T, O> {}

impl<T, O> Mode<'_, T, O> {
    #[inline(always)]
    fn values(&self) -> *mut Values<T, O> {
        match *self {
            Mode::Pinned {
                value,
                ..
            } => value,
            #[cfg(feature = "alloc")]
            // SOUND: (valid-ptr) Not-pub, and is always valid at instantiation.
            //
            // SOUND: (use-after-free) Not public type. Encapsulating type owns it.
            //
            // SOUND: (no exclusive ref violation)
            // * `*const ptr` never borrowed exclusively
            // * ptr never leaked
            //
            // NEED: erasing Cycler's storage generic, which ends up recursive
            Mode::Boxed(ptr) => unsafe { &*addr_of!((*ptr).interchange) }.get()
        }
    }

    #[inline(always)]
    /// Assumes caller is responsible for an Rc (strong)
    pub fn next(&self) -> Option<(T, *mut Option<O>)> {
        // SOUND: (valid-ptr) Not-pub, and is always valid at instantiation.
        //
        // SOUND: (use-after-free) Not public type.
        // Either encapsulating type owns it, or reflected in lifetime.
        //
        // SOUND: (&mut exclusive)
        // * only accessed in this impl
        // * non-recursively (note: no calls to drop)
        // * behind UnsafeCell
        // * !Send, !Sync
        //
        // NEED: lock-free exchange
        // NEED: pinned-variant's lifetime cheat
        unsafe { &mut *self.values() }.next_inner()
    }

    #[inline(always)]
    /// Requires checking strong().
    pub unsafe fn push(&self, value: T, passback: *mut Option<O>) {
        // SOUND: (valid-ptr) Not-pub, and is always valid at instantiation.
        //
        // SOUND: (use-after-free) Not public type.
        // Reflected in lifetime, or by strong()
        //
        // SOUND: (&mut exclusive)
        // * only accessed in this impl
        // * non-recursively (note: drop is after exclusive-reference is gone for no-alloc)
        // * behind UnsafeCell
        // * !Send, !Sync
        //
        // NEED: lock-free exchange
        // NEED: pinned-variant's lifetime cheat
        let _ = (&mut *self.values()).push_inner(value, passback);
    }

    #[inline(always)]
    /// Requires checking strong().
    pub unsafe fn remove(&self, passback: *mut Option<O>) {
        (&mut *self.values()).remove(passback);
    }

    #[inline(always)]
    #[cfg(feature = "alloc")]
    /// Assumes caller is responsible for a Weak.
    // SOUND: (use-after-free) cannot be called after dropping()
    //
    // SOUND: (no exclusive ref violation)
    // * `*const ptr`s never borrowed exclusively
    // * ptrs never leaked
    //
    // NEED: erasing Cycler's storage generic, which ends up recursive
    // NEED: use-after-free prevention of value-exchange
    pub fn strong(&self) -> bool {
        if let &Mode::Boxed(ptr) = self {
            unsafe { References::strong(ptr) }
        } else {
            true
        }
    }

    #[cfg(not(feature = "alloc"))]
    pub const fn strong(&self) -> bool {
        true
    }

    /// Assumes caller is responsible for an Rc (strong)
    pub fn len_upper_bound(&self) -> usize {
        use Values::*;
        match unsafe { &*self.values() } {
            Present(_, _) => 1,
            Missing
            | Waiting(_)
                => 0,
            #[cfg(feature = "alloc")]
            Multiple(list) => list.len(),
        }
    }
}
