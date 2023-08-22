use core::future::Future;

use super::super::Remit;

impl<T, O> Remit<'_, T, O> {
    /// Remits the value to the iterator.
    ///
    /// If multiple calls are performed without awaiting for the iterator to consume them,
    /// an unbounded buffer will be allocated to store the extra values.
    /// Only available with the `alloc` feature, otherwise behavior is SemVer
    /// [unspecified](https://doc.rust-lang.org/reference/behavior-not-considered-unsafe.html),
    /// but currently replaces the previous value.
    ///
    /// A caller *should* await the future, but does not need to.
    /// The provided future will only finish when all values have been accepted by the iterator.
    ///
    /// The provided future does not awake on the iterator consuming values;
    /// the iterator will poll the originally created future unilaterally.
    ///
    /// If the iterator has been dropped,
    /// values will be discarded and the future(s) will always poll as pending.
    #[inline(always)]
    pub fn value<'a>(&'a self, value: T) -> impl Future<Output=O> + 'a {
        Self::impl_value(self.0, value)
    }
}
