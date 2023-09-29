use core::{
    future::Future,
    pin::Pin,
    task::{
        Context,
        Poll::{
            self,
            *,
        },
    }
};

use super::super::{
    GeneratorIterator,
    GeneratorIterNext,
    Exchange,
};

impl<T, P, O, F> Unpin for GeneratorIterator<'_, T, P, O, F> {}

impl<'s, T, P: Future<Output=()>, O: 's, F: FnMut() -> O> GeneratorIterator<'s, T, P, F, O> {
    /// Allows passing in a [`Context`] so that nested async/await-calls can be used.
    pub fn poll_next_item(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let Ready(value) = self.generator.impl_poll_next(cx)
            else { return Pending };
        let Some(
            Exchange {
                value,
                passback,
            }
        ) = value
            else { return Ready(None) };
        passback.provide((self.provider)());
        Ready(Some(value))
    }

    /// Wraps [`poll_next_item()`](Self::poll_next_item()) in a [`Future`] that can be awaited.
    pub fn next_item_future(&mut self) -> GeneratorIterNext<'_, 's, T, P, F, O> {
        GeneratorIterNext(self)
    }
}

impl<'s, T, P: Future<Output=()>, O: 's, F: FnMut() -> O> Iterator for GeneratorIterator<'s, T, P, F, O> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(
            Exchange {
                value,
                passback,
            }
        ) = self.generator.next()
            else { return None };
        passback.provide((self.provider)());
        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.generator.size_hint()
    }
}
