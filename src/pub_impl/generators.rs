use core::{
    pin::Pin,
    cell::UnsafeCell,
    marker::PhantomPinned,
};

use super::super::{
    Generators,
    GeneratorIterator,
    RemitWithLifetime,
    Remit,
    Values,
    Generator,
};

impl<T, P, O> Generators<T, P, O> {
    /// Provides the storage to be pinned when not using an allocation.
    pub fn new() -> Self {
        Generators {
            values: UnsafeCell::new(Values::Missing),
            future: None,
            _pin: PhantomPinned,
        }
    }
}

impl<T, P, O> Generators<T, P, O> {
    #[allow(clippy::needless_lifetimes)]
    #[inline(always)]
    pub fn pinned_exchange<'s, G>(
        self: Pin<&'s mut Self>,
        gen: G,
    ) -> Generator<'s, T, P, O>
        where
            // insures fn is not implemented only for 'static
            G: RemitWithLifetime<T, O, ()>,
            // insures P is properly defined, even if it actually has a lifetime
            G: FnOnce(Remit<'static, T, O>) -> P,
            O: 's,
    {
        self.impl_pinned_exchange(gen)
    }

    #[inline(always)]
    pub fn parameterized_exchange<'s, G, X>(
        self: Pin<&'s mut Self>,
        gen: G,
        parameter: X,
    ) -> Generator<'s, T, P, O>
        where
            // insures fn is not implemented only for 'static
            G: RemitWithLifetime<T, O, (X,)>,
            // insures P is properly defined, even if it actually has a lifetime
            G: FnOnce(X, Remit<'static, T, O>) -> P,
            O: 's,
    {
        self.impl_parameterized_exchange(gen, parameter)
    }

    #[cfg(feature = "alloc")]
    #[inline(always)]
    pub fn boxed_exchange(gen: impl FnOnce(Remit<'static, T, O>) -> P) -> Generator<'static, T, P, O> {
        Self::impl_boxed_exchange(gen)
    }
}

impl<T, P, O: Default> Generators<T, P, O> {
    #[allow(clippy::needless_lifetimes)]
    #[inline(always)]
    /// Takes the pinned storage and the generator and provides an iterator.
    /// Stack based (does not use an allocation).
    ///
    /// The internal storage assumes the generator was valid for a provided `'static`,
    /// but requires the generator to be valid for all provided lifetimes.
    /// That is, the `Remit` provided to the generator cannot be moved out,
    /// even if at first glance it appears the storage does not have that restriction.
    /// In effect, this relaxes the lifetime requirements of the storage,
    /// but not the provided generator.
    ///
    /// Uses the default value for exchange, which is implicitly unit.
    pub fn of<'s, G>(
        self: Pin<&'s mut Self>,
        gen: G,
    ) -> GeneratorIterator<'s, T, P, impl Fn() -> O, O>
        where
            // insures fn is not implemented only for 'static
            G: RemitWithLifetime<T, O, ()>,
            // insures P is properly defined, even if it actually has a lifetime
            G: FnOnce(Remit<'static, T, O>) -> P,
    {
        self.impl_pinned_exchange(gen).defaults()
    }

    #[allow(clippy::needless_lifetimes)]
    #[inline(always)]
    /// The same as [`Generators::of()`] but allows passing a parameter in.
    pub fn parameterized<'s, G, X>(
        self: Pin<&'s mut Self>,
        gen: G,
        parameter: X,
    ) -> GeneratorIterator<'s, T, P, impl Fn() -> O, O>
        where
            // insures fn is not implemented only for 'static
            G: RemitWithLifetime<T, O, (X,)>,
            // insures P is properly defined, even if it actually has a lifetime
            G: FnOnce(X, Remit<'static, T, O>) -> P,
    {
        self.impl_parameterized_exchange(gen, parameter).defaults()
    }

    #[cfg(feature = "alloc")]
    #[inline(always)]
    /// Uses an allocation so that the iterator does not need to be borrowed.
    /// Useful for returning an iterator from a function, where it can't be pinned to the stack.
    ///
    /// The generator only needs to be valid for `'static`; it does not need to be valid for all lifetimes.
    ///
    /// To pass in parameters, use a capturing closure.
    ///
    /// Uses the [`Default::default()`] value for exchange, which is implicitly [unit].
    pub fn boxed(gen: impl FnOnce(Remit<'static, T, O>) -> P) -> GeneratorIterator<'static, T, P, impl Fn() -> O, O> {
        Self::impl_boxed_exchange(gen).defaults()
    }
}
