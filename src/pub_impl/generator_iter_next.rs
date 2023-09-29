use core::{
    future::Future,
    task::{
        Context,
        Poll,
    },
    pin::Pin,
};

use super::super::GeneratorIterNext;

impl<T, P, O, F> Unpin for GeneratorIterNext<'_, '_, T, P, F, O> {}

impl<'a, 's, T, P: Future<Output=()>, O: 's, F: FnMut() -> O> Future for GeneratorIterNext<'a, 's, T, P, F, O> {
    type Output = Option<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut *self.0).poll_next_item(cx)
    }
}
