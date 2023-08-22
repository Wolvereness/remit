use core::{
    future::Future,
    marker::PhantomData,
};

use super::super::{
        Exchange,
        Generator,
        Indirection,
        RemitBack,
        internal_impl::mode::Mode,
};

#[cfg(feature = "alloc")]
use alloc::rc::Rc;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use crate::context;

impl<'s, T, P: Future<Output=()>, O: 's> Generator<'s, T, P, O> {
    pub(crate) fn make_exchange(&mut self, entry: (T, *mut Option<O>)) -> Exchange<'s, T, O> {
        let (value, passback) = entry;
        let (indirection, indirection_ctx) = match self.mode {
            Mode::Pinned { value, .. } =>
                (
                    RemitBack::<O>::indirection_stack::<T> as Indirection<'s, O>,
                    value as *const (),
                ),
            #[cfg(feature = "alloc")]
            Mode::Boxed(references) => {
                let _ = Rc::downgrade(unsafe { self.owner.as_ref().unwrap_unchecked() }).into_raw();
                (
                    RemitBack::<O>::indirection_boxed::<T> as Indirection<'s, O>,
                    references as *const (),
                )
            },
        };
        Exchange {
            value,
            passback: RemitBack {
                indirection,
                indirection_ctx,
                data: passback,
                _ref: PhantomData,
            },
        }
    }

    pub(crate) fn impl_next(&mut self) -> Option<Exchange<'s, T, O>> {
        if let Some(entry) = self.mode.next() {
            return Some(self.make_exchange(entry));
        }
        if self.done {
            return None
        }
        // FIXME: https://github.com/rust-lang/rust/issues/102012
        // SOUND: We can't use Arc without alloc,
        // so context just defines some no-operation functions to fill out a v-table.
        let waker = unsafe { Waker::from_raw(context::NOOP_WAKER) };
        // SOUND: (pinning) Sound, we created the ptr to future ourselves and it was pinned,
        // either via Rc or via a pinned-self.
        //
        // SOUND: (&mut exclusive) The ptr never gets touched other than here,
        // or after the lifetime expires and Generator is re-borrowed.
        // Note that this is through an exclusive-borrow of self.
        // Original exclusive-reference was also used for creation without leaking.
        //
        // SOUND: (use-after-free) The ptr's lifetime is reflected in GeneratorIterator,
        // either owned in _owner, or pinned-self.
        //
        // SOUND: (valid-ptr) Not-pub, and is always valid at instantiation.
        if let Poll::Ready(()) = unsafe { Pin::new_unchecked(&mut *self.future) }.poll(&mut Context::from_waker(&waker)) {
            self.done = true;
        }
        Some(self.make_exchange(self.mode.next()?))
    }
}
