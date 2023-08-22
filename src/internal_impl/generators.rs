use core::{
    pin::Pin,
    marker::PhantomData,
};

#[cfg(feature = "alloc")]
use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
};

#[cfg(feature = "alloc")]
use alloc::{
    rc::Rc,
};

use super::super::{
    Generator,
    Generators,
    RemitWithLifetime,
    Remit,
    Mode,
};

#[cfg(feature = "alloc")]
use super::super::{
    Cycler,
    References,
};

impl<T, P, O> Generators<T, P, O> {
    pub(crate) fn impl_pinned_exchange<'s, G>(
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
        // SOUND: Pin passthrough; only `future` is inner-pinned.
        // `future` only ever gets replaced via Option::insert
        let inner = unsafe { self.get_unchecked_mut() };
        let value = inner.values.get();
        let mode = Mode::Pinned {
            value,
            // This becomes 'static, and the trait-guard is where the real protection is
            _lifetime: PhantomData,
        };
        let future = gen(Remit(mode));
        let future = inner.future.insert(future);
        Generator {
            done: false,
            mode,
            future,
            #[cfg(feature = "alloc")]
            owner: None,
        }
    }

    pub(crate) fn impl_parameterized_exchange<'s, G, X>(
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
        // SOUND: Pin passthrough; only `future` is inner-pinned.
        // `future` only ever gets replaced via Option::insert
        let inner = unsafe { self.get_unchecked_mut() };
        let value = inner.values.get();
        let mode = Mode::Pinned {
            value,
            // This becomes 'static, and the trait-guard is where the real protection is
            _lifetime: PhantomData,
        };
        let future = gen(parameter, Remit(mode));
        let future = inner.future.insert(future);
        Generator {
            done: false,
            mode,
            future,
            #[cfg(feature = "alloc")]
            owner: None,
        }
    }

    #[cfg(feature = "alloc")]
    pub(crate) fn impl_boxed_exchange(gen: impl FnOnce(Remit<'static, T, O>) -> P) -> Generator<'static, T, P, O> {
        let rc = Rc::new(Cycler {
            future: Default::default(),
            references: References::new::<P>(),
            weak_inner: UnsafeCell::new(MaybeUninit::uninit()),
            _pin: Default::default(),
        });
        let weak = Rc::downgrade(&rc);
        // SOUND: Writing to an UnsafeCell.
        // Only spot where it's being written, having been freshly created.
        //
        // NEED: unsafe-cell lets shared-references to not conflict with exclusive-reference to weak_inner
        let ptr = unsafe { &mut *rc.weak_inner.get() }.write(weak);
        // SOUND: no re-use of ptr
        // SOUND: !Send !Sync respected, via `*mut P` in GeneratorIterator.
        rc.references.ptr.set(unsafe { Cycler::<P, T, O>::ptr_convert(ptr) });

        let mode = Mode::Boxed(&rc.references);
        // SOUND: Writing to an UnsafeCell.
        // Only spot where it's being written, having been freshly created.
        //
        // NEED: unsafe-cell lets shared-references to not conflict with exclusive-reference to future
        let future = unsafe { &mut *rc.future.get() }.insert(gen(Remit(mode)));

        Generator {
            done: false,
            mode,
            future,
            owner: Some(rc),
        }
    }
}
