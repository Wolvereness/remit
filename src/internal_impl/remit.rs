use core::{
    cell::UnsafeCell,
    marker::PhantomPinned,
};

use super::super::{
    Remit,
    Mode,
};

#[cfg(feature = "alloc")]
use super::super::References;

mod remit_future;

enum ExchangeState<T, O> {
    Waiting(T),
    // cannot make a finished state, because pinning
    Provided(UnsafeCell<Option<O>>, PhantomPinned),
}

impl<T, O> Remit<'_, T, O> {
    pub(crate) fn impl_value(mode: Mode<'_, T, O>, value: T) -> remit_future::RemitFuture<'_, T, O> {
        remit_future::RemitFuture {
            exchange: ExchangeState::Waiting(value),
            mode,
        }
    }
}

#[cfg(feature = "alloc")]
impl<T, O> Drop for Remit<'_, T, O> {
    fn drop(&mut self) {
        if let &mut Remit(Mode::Boxed(ptr)) = self {
            // SOUND: Remit was constructed with a single Weak
            unsafe { References::dropping(ptr) }
        }
    }
}
