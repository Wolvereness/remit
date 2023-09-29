use core::{
    future::Future,
    marker::{
        PhantomData,
        Unpin,
    },
    pin::Pin,
    task::{
        Context,
        Poll::{
            self,
            *,
        },
        Waker,
    },
};

use super::super::{
    Exchange,
    Generator,
    RemitBack,
    internal_impl::mode::Mode,
    context,
};

impl<T, P, O> Unpin for Generator<'_, T, P, O> {}

impl<'s, T, P: Future<Output=()>, O: 's> Generator<'s, T, P, O> {
    pub(crate) fn make_exchange(&mut self, entry: (T, *mut Option<O>)) -> Exchange<'s, T, O> {
        let (value, passback) = entry;
        let (indirection, indirection_ctx) = match self.mode {
            Mode::Pinned { value, .. } =>
                RemitBack::<O>::indirection_stack_ptr::<'s, T>(value),
            #[cfg(feature = "alloc")]
            Mode::Boxed(references) =>
                // SOUND: Boxed mode is allocated, which means owner is-some
                unsafe { RemitBack::<O>::indirection_boxed_ptr::<T, P>(references, &self.owner) },
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

    #[inline(always)]
    pub(crate) fn impl_next(&mut self) -> Option<Exchange<'s, T, O>> {
        // FIXME: https://github.com/rust-lang/rust/issues/102012
        // SOUND: We can't use Arc without alloc,
        // so context just defines some no-operation functions to fill out a v-table.
        let waker = unsafe { Waker::from_raw(context::NOOP_WAKER) };
        let Ready(value) = self.impl_poll_next(&mut Context::from_waker(&waker))
            else { return None };
        value
    }

    pub(crate) fn impl_poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Exchange<'s, T, O>>> {
        if let Some(entry) = self.mode.next() {
            return Ready(Some(self.make_exchange(entry)));
        }
        if self.done {
            return Ready(None)
        }
        // SOUND: (pinning) Sound, we created the ptr to future ourselves and it was pinned,
        // either via Rc or via a pinned-self.
        //
        // SOUND: (&mut exclusive) The ptr never gets touched other than here,
        // or after the lifetime expires and Generator is re-borrowed.
        // Note that this is through an exclusive-borrow of self.
        // Original exclusive-reference was also used for creation without leaking.
        //
        // SOUND: (use-after-free) The ptr's lifetime is reflected in GeneratorIterator,
        // either owned in owner, or pinned-self.
        //
        // SOUND: (valid-ptr) Not-pub, and is always valid at instantiation.
        if let Ready(()) = unsafe { Pin::new_unchecked(&mut *self.future) }.poll(cx) {
            self.done = true;
        }
        if let Some(value) = self.mode.next() {
            Ready(Some(self.make_exchange(value)))
        } else if self.done {
            Ready(None)
        } else {
            Pending
        }
    }
}
