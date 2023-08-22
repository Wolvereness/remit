use core::{
    cell::UnsafeCell,
    marker::PhantomPinned,
    mem::MaybeUninit,
    ptr::read
};

use alloc::rc::Weak;

use super::super::References;

pub struct Cycler<P, T, O> {
    pub future: UnsafeCell<Option<P>>,
    pub references: References<T, O>,
    pub weak_inner: UnsafeCell<MaybeUninit<Weak<Cycler<P, T, O>>>>,
    pub _pin: PhantomPinned,
}

impl<P, T, O> Cycler<P, T, O> {
    #[inline(always)]
    /// Exclusive-ref must not reused.
    /// Resulting ptr must be kept !Send !Sync
    // NEED: erasing Cycler's storage generic, which ends up recursive
    pub unsafe fn ptr_convert(ptr: &mut Weak<Cycler<P, T, O>>) -> *mut () {
        ptr as *mut _ as _
    }

    /// ptr must be created with this Cycler's ptr_convert.
    /// May only be called once.
    // NEED: erasing Cycler's storage generic, which ends up recursive
    pub unsafe fn do_inner_drop(ptr: *mut ()) {
        let ptr: *mut Weak<Cycler<P, T, O>> = ptr as _;
        // SOUND: (Rc-race-condition) ptr_convert requires !Send !Sync
        // SOUND: (valid-ptr) ptr_convert instantiation
        // SOUND: (double-drop) can only be called once
        let _: Weak<Cycler<P, T, O>> = read(ptr);
    }

    /// ptr must be created with this Cycler's ptr_convert.
    /// Must not be called after do_inner_drop.
    // NEED: erasing Cycler's storage generic, which ends up recursive
    pub unsafe fn is_strong(ptr: *mut ()) -> bool {
        let ptr: *const Weak<Cycler<P, T, O>> = ptr as _;
        // SOUND: (use-after-free) can't be called after do_inner_drop
        // SOUND: (valid-ptr) ptr_convert instantiation
        // SOUND: (no exclusive ref violation) only exclusive-ref is do_inner_drop
        (*ptr).strong_count() > 0
    }
}
