use core::{
    future::Future,
    task::{
        Context,
        Poll,
    },
    pin::Pin,
};

use super::super::{
    Exchange,
    GeneratorNext,
};

impl<T, P, O> Unpin for GeneratorNext<'_, '_, T, P, O> {}

impl<'a, 's, T, P: Future<Output=()>, O: 's> Future for GeneratorNext<'a, 's, T, P, O> {
    type Output = Option<Exchange<'s, T, O>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.impl_poll_next(cx)
    }
}
