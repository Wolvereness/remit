use core::{
    marker::PhantomPinned,
    hint::unreachable_unchecked,
    future::Future,
    cell::UnsafeCell,
    mem,
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use super::{
    ExchangeState,
    super::super::Mode,
};

pub(crate) struct RemitFuture<'a, T, O> {
    pub(super) exchange: ExchangeState<T, O>,
    pub(super) mode: Mode<'a, T, O>,
}

impl<T, O> Future for RemitFuture<'_, T, O> {
    type Output = O;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SOUND: only Provided is projected, and never over-written
        let this = unsafe { self.get_unchecked_mut() };
        if let ExchangeState::Provided(provided, _) = &this.exchange {
            // SOUND: no recursive drop, !Send, !Sync
            if let Some(value) = unsafe { &mut *provided.get() }.take() {
                return Poll::Ready(value);
            }
            return Poll::Pending;
        }
        let ExchangeState::Waiting(value) = mem::replace(
            &mut this.exchange,
            ExchangeState::Provided(UnsafeCell::new(None), PhantomPinned)
        )
            else {
                // SOUND: note above state, by exclusion
                unsafe { unreachable_unchecked() };
            };
        let ExchangeState::Provided(cell, _) = &this.exchange
            else {
                // SOUND: note above assignment
                unsafe { unreachable_unchecked() }
            };
        let ptr = cell.get();
        if this.mode.strong() {
            // SOUND: strong checked
            unsafe { this.mode.push(value, ptr); }
        }

        Poll::Pending
    }
}

impl<T, O> Drop for RemitFuture<'_, T, O> {
    fn drop(&mut self) {
        let ExchangeState::Provided(cell, _) = &self.exchange
            else {
                return
            };
        let ptr = cell.get();
        // SOUND: no recursive drop, !Send, !Sync
        if unsafe { &*ptr }.is_some() || !self.mode.strong() {
            return
        }
        // SOUND: strong checked
        unsafe { self.mode.remove(ptr) };
    }
}
