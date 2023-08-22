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
    #[inline(always)]
    pub fn value<'a>(&'a self, value: T) -> impl Future<Output=O> + 'a {
        Self::impl_value(self.0, value)
    }
}
