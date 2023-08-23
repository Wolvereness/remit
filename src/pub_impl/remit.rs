use core::future::Future;

use super::super::Remit;

impl<T, O> Remit<'_, T, O> {
    /// Remits the value to the iterator.
    ///
    /// If multiple calls are awaited together,
    /// an unbounded buffer will be allocated to store the extra values.
    /// Only available with the `alloc` feature, otherwise behavior is SemVer
    /// [unspecified](https://doc.rust-lang.org/reference/behavior-not-considered-unsafe.html),
    /// but currently replaces the previous value.
    ///
    /// A caller *should* await the future, or the value will not be sent.
    /// If the `Future` is dropped before the value has been pulled by the iterator,
    /// it will not be sent.
    /// The `Future` will be ready once the [`Exchange`](crate::Exchange) sends back a value.
    /// Specifically, the first [`Future::poll()`] will send the value.
    ///
    /// The provided future does not [wake](core::task::Waker::wake())
    /// on the `Exchange` sending back values;
    /// the iterator will poll the originally created future unilaterally.
    ///
    /// If the iterator has been dropped,
    /// values will be discarded and the future(s) will always poll as pending.
    ///
    /// # Consistency Warning
    ///
    /// If the future is polled, but dropped after the value is received, and a new future is
    /// polled before the [`RemitBack`](super::super::RemitBack) or
    /// [`Exchange`](super::super::Exchange) are handled, the provided value may be mis-matched.
    ///
    /// ```
    /// use std::pin::pin;
    /// use std::future::{Future, poll_fn};
    /// use std::task::Poll;
    /// use remit::*;
    ///
    /// async fn exchange_oddity(remit: Remit<'_, String, String>) {
    ///     let mut future = pin!(remit.value("GenValue1".into()));
    ///     let mut poll_count = 0;
    ///     poll_fn(|ctx| {
    ///         match poll_count {
    ///             0 => {
    ///                 poll_count += 1;
    ///                 let _ = future.as_mut().poll(ctx);
    ///             },
    ///             1 => {
    ///                 poll_count += 1;
    ///                 future.set(remit.value("GenValue2".into()));
    ///                 let _ = future.as_mut().poll(ctx);
    ///             },
    ///             2 => {
    ///                 poll_count += 1;
    ///                 let Poll::Ready(mut exchanged) = future.as_mut().poll(ctx)
    ///                     else { panic!("Unexpected test polling") };
    ///                 exchanged.push_str(" - Exchanged for GenValue2");
    ///                 future.set(remit.value(exchanged));
    ///                 let _ = future.as_mut().poll(ctx);
    ///             },
    ///             _ => return Poll::Ready(()),
    ///         };
    ///         Poll::Pending
    ///     }).await
    /// }
    ///
    /// let gen = pin!(Generators::new());
    /// let mut gen = gen.pinned_exchange(exchange_oddity);
    /// let exchange1 = gen.next().unwrap();
    /// let exchange2 = gen.next().unwrap();
    ///
    /// let mut results = vec![
    ///     exchange1.provide("ExValue1".into()),
    ///     exchange2.provide("ExValue2".into()),
    ///     gen.next().unwrap().as_ref().clone(),
    /// ];
    /// assert_eq!(
    ///     results,
    ///     vec![
    ///         "GenValue1".to_string(),
    ///         "GenValue2".to_string(),
    ///         // Notice that in response to GenValue2,
    ///         // it sent ExValue1,
    ///         // which was supposed to be respective to GenValue1.
    ///         "ExValue1 - Exchanged for GenValue2".to_string(),
    ///     ],
    /// );
    /// ```
    #[inline(always)]
    pub fn value<'a>(&'a self, value: T) -> impl Future<Output=O> + 'a {
        Self::impl_value(self.0, value)
    }
}
