//! Generators implemented through async/await syntax.
//!
//! The pinned implementation is stack-based, and the boxed is heap-based.
//! No fancy macros and a simple API. Values can be lazily or eagerly yielded.
//!
//! This crate is inherently no-std, and the default `alloc` feature can be disabled.
//!
//! Some behaviors exhibited by the *lack* of `alloc` are not part of the SemVer.
//! For example, not awaiting before another remit, without alloc, is
//! [unspecified](https://doc.rust-lang.org/reference/behavior-not-considered-unsafe.html)
//! behavior.
//!
//! ## Examples
//!
//! General usage of unbounded generator.
//! ```
//! use std::pin::pin;
//! use remit::{Generators, Remit};
//!
//! async fn gen(remit: Remit<'_, usize>) {
//!     remit.value(42).await;
//!     // Does not need to be limited
//!     for i in 1.. {
//!         remit.value(i).await
//!     }
//! }
//! for item in pin!(Generators::new()).of(gen).take(10) {
//!     println!("{item}");
//!     // Prints 42, 1, 2, 3, 4, 5, 6, 7, 8, 9
//! }
//! assert_eq!(vec![42, 1, 2, 3], pin!(Generators::new()).of(gen).take(4).collect::<Vec<_>>());
//! /*
//! // Rust has trouble determining the lifetime
//! assert_eq!(
//!     vec![1],
//!     pin!(Generators::new())
//!         .of(|remit: Remit<'_, usize>| async move { remit.value(1).await; })
//!         .collect::<Vec<_>>(),
//! );
//! */
//! # #[cfg(feature = "alloc")]
//! assert_eq!(vec![42, 1], Generators::boxed(gen).take(2).collect::<Vec<_>>());
//! # #[cfg(feature = "alloc")]
//! fn iter() -> impl Iterator<Item=usize> {
//!     Generators::boxed(gen)
//! }
//! ```
//!
//! Parameterized usage.
//! ```
//! # use std::pin::pin;
//! # use remit::{Generators, Remit};
//! # use std::fmt;
//! # #[cfg(feature = "alloc")] {
//! async fn scream<D: fmt::Display>(iter: impl Iterator<Item=D>, remit: Remit<'_, String>) {
//!     for person in iter {
//!         remit.value(format!("{person} scream!")).await
//!     }
//!     remit.value("... for ice cream!".to_string()).await;
//! }
//! let expected: Vec<String> = ["You scream!", "I scream!", "We all scream!", "... for ice cream!"].iter().map(ToString::to_string).collect();
//! assert_eq!(
//!     expected,
//!     pin!(Generators::new()).parameterized(scream, ["You", "I", "We all"].iter()).collect::<Vec<String>>(),
//! );
//! assert_eq!(
//!     expected,
//!     Generators::boxed(|remit| scream(["You", "I", "We all"].iter(), remit)).collect::<Vec<String>>(),
//! );
//! # }
//! ```
//!
//! Usage of a generator that only functions for `'static`.
//! ```
//! # use remit::{Generators, Remit};
//! # #[cfg(feature = "alloc")] {
//! async fn gen(remit: Remit<'static, usize>) {
//!     remit.value(2).await;
//!     remit.value(3).await;
//!     remit.value(5).await;
//!     remit.value(7).await;
//! }
//! for item in Generators::boxed(gen) {
//!     println!("{item}");
//! }
//! assert_eq!(vec![2, 3, 5, 7], Generators::boxed(gen).collect::<Vec<_>>());
//! assert_eq!(
//!     vec![1],
//!     Generators::boxed(|remit| async move {
//!         // Note that `let () =` helps determine what the RemitBack type is.
//!         // That is, it doesn't detect default `()` for non-exchangers.
//!         let () = remit.value(1).await;
//!     }).collect::<Vec<_>>(),
//! );
//!
//! fn iter() -> impl Iterator<Item=usize> {
//!     Generators::boxed(gen)
//! }
//! # }
//! ```
//!
//! Usage of generators that exchange values.
//! ```
//! # use remit::{Generators, Remit};
//! # use std::pin::pin;
//! async fn health_regen((starting, regen, minimum): (usize, usize, usize), remit: Remit<'_, usize, usize>) {
//!     let mut current = starting;
//!     while current >= minimum {
//!         let damage = remit.value(current).await;
//!         current -= damage;
//!         current += regen;
//!     }
//! }
//!
//! let mut buffer = vec![];
//! for exchange in pin!(Generators::new()).parameterized_exchange(health_regen, (400, 3, 20)) {
//!     let (previous, to_damage) = exchange.take();
//!     buffer.push(previous);
//!     to_damage.provide(previous * 2 / 7);
//! }
//! assert_eq!(
//!     buffer,
//!     vec![400, 289, 210, 153, 113, 84, 63, 48, 38, 31, 26, 22],
//! );
//!
//!
//! ```
//!
//! Unorthodox usage of yielding values.
//! ```
//! # use std::future::{Future, poll_fn};
//! # use std::pin::pin;
//! # use std::task::Poll;
//! # use remit::{Generators, Remit};
//! // These implementations run successfully.
//! // However, they trigger creation of a buffer with alloc.
//! async fn no_await(remit: Remit<'_, usize>) {
//!     let mut a = pin!(remit.value(2));
//!     let mut a_done = false;
//!     let _ = remit.value(3); // Never gets yielded
//!     let mut b = pin!(remit.value(5));
//!     let mut b_done = false;
//!     let _ = remit.value(7).await;
//!     let mut c = pin!(remit.value(11));
//!     let mut c_done = false;
//!     let mut d = pin!(remit.value(13));
//!     poll_fn(|ctx| {
//!         // Note proper usage of not polling again after ready.
//!         if !a_done {
//!             if let Poll::Ready(()) = a.as_mut().poll(ctx) {
//!                 a_done = true;
//!             }
//!         }
//!         if !b_done {
//!             if let Poll::Ready(()) = b.as_mut().poll(ctx) {
//!                 b_done = true;
//!             }
//!         }
//!         if !c_done {
//!             if let Poll::Ready(()) = c.as_mut().poll(ctx) {
//!                 c_done = true;
//!             }
//!         }
//!         // Without alloc, a & b & c were pushed out after being polled.
//!         d.as_mut().poll(ctx)
//!     }).await;
//!     remit.value(17).await;
//!     if !a_done {
//!         a.await;
//!     }
//!     // Notice that this value never gets yielded without alloc!
//!     // `a` was polled early, but the other polls pushed it out.
//!     // Because the backing was pushed out, `a` was never ready.
//!     remit.value(42).await;
//! }
//! assert_eq!(
//!     if cfg!(feature = "alloc") {
//!         vec![7, 2, 5, 11, 13, 17, 42]
//!     } else {
//!         vec![7, 13, 17]
//!     },
//!     pin!(Generators::new()).of(no_await).collect::<Vec<_>>(),
//! );
//!
//! async fn delay_await(remit: Remit<'_, usize>) {
//!     let first_remit = remit.value(11);
//!     remit.value(13).await;
//!     // Sends the value
//!     first_remit.await;
//!
//!     let _ = remit.value(17);
//!     let _ = remit.value(19);
//!     // Values were not polled/awaited, and were not sent.
//! }
//! assert_eq!(
//!     vec![13, 11],
//!     pin!(Generators::new()).of(delay_await).collect::<Vec<_>>()
//! );
//! ```
//!
//! Usage of a boxed generator that borrows the parameter.
//! ```
//! # use remit::*;
//! # #[cfg(feature = "alloc")] {
//! let data = String::from("hi");
//!
//! async fn gen_implicit(data: &str, remit: Remit<'static, usize>) {
//!     remit.value(data.len()).await;
//!     remit.value(data.len()).await;
//! }
//!
//! fn gen_explicit<'a>(data: &'a str, remit: Remit<'static, usize>) -> impl std::future::Future<Output=()> + 'a {
//!     async move {
//!         remit.value(data.len()).await;
//!         remit.value(data.len()).await;
//!     }
//! }
//!
//! fn iter_explicit<'a>(data: &'a str) -> GeneratorIterator<'static, usize, impl std::future::Future<Output=()> + 'a, impl Fn() -> () + 'a> {
//!     Generators::boxed(|remit| gen_explicit(data, remit))
//! }
//!
//! fn iter_implicit(data: &str) -> GeneratorIterator<'static, usize, impl std::future::Future<Output=()> + '_, impl Fn() -> () + '_> {
//!     Generators::boxed(|remit| gen_implicit(data, remit))
//! }
//!
//! for item in iter_explicit(&data) {
//!     dbg!(item);
//! }
//! for item in iter_implicit(&data) {
//!     dbg!(item);
//! }
//! for item in Generators::boxed(|remit| gen_explicit(&data, remit)) {
//!     dbg!(item);
//! }
//! for item in Generators::boxed(|remit| gen_implicit(&data, remit)) {
//!     dbg!(item);
//! }
//! # }
//! ```
//!
//! Usage of a stack-based parameterized generator that borrows the parameter.
//! ```
//! # use std::pin::pin;
//! # use remit::*;
//! let data = String::from("hi");
//!
//! async fn gen_implicit(data: &str, remit: Remit<'_, usize>) {
//!     remit.value(data.len()).await;
//!     remit.value(data.len()).await;
//! }
//! for item in pin!(Generators::new()).parameterized(gen_implicit, &data) {
//!     dbg!(item);
//! }
//!
//! /// Does not work, as explicit lifetime definitions fail HRTB.
//! fn gen_explicit<'a: 'c, 'b: 'c, 'c>(data: &'a str, remit: Remit<'b, usize>) -> impl std::future::Future<Output=()> + 'c {
//!     async move {
//!         remit.value(data.len()).await;
//!         remit.value(data.len()).await;
//!     }
//! }
//! /* // See note above and https://github.com/rust-lang/rust/issues/114947
//! for item in pin!(Generators::new()).parameterized(gen_explicit, &data) {
//!     dbg!(item);
//! }
//! */
//! ```
//!
//! Incorrect attempt of a stack-based generator.
//! ```compile_fail
//! # use std::pin::pin;
//! # use remit::{Generators, Remit};
//! /// Only accepts `'static`, so it needs to be boxed.
//! async fn gen(remit: Remit<'static, usize>) {
//!     remit.value(1).await;
//! }
//! // Fails to compile, because gen is only `'static` and pinning is for the stack.
//! for item in pin!(Generators::new()).of(gen) {
//!     println!("{item}");
//! }
//! ```
//!
//! ## Features
//!
//! * **alloc** -
//!   Enables the use of a boxed generator and multiple pending values.
//!   Defaults to enabled.

use core::{
    cell::UnsafeCell,
    future::{
        Future,
    },
    hint::unreachable_unchecked,
    marker::{
        PhantomData,
        PhantomPinned,
    },
    mem::{
        self,
        ManuallyDrop,
    },
    pin::Pin,
    task::{
        Poll,
        Context,
        Waker,
    },
    ptr::{
        eq,
        write,
    }
};

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use core::{
    cell::Cell,
    mem::MaybeUninit,
    ptr::{
        null_mut,
        read,
        addr_of,
    },
};

#[cfg(feature = "alloc")]
use alloc::{
    collections::VecDeque,
    rc::{
        Rc,
        Weak,
    },
};

mod context;

/// Erases the return-type so that other parameters don't get polluted by the HRTB.
trait AsyncFnOnce<Arg> {}

impl<F, A, R: Future> AsyncFnOnce<(A, )> for F
    where
        F: FnOnce(A) -> R,
{}

impl<F, A, B, R: Future> AsyncFnOnce<(A, B, )> for F
    where
        F: FnOnce(A, B) -> R,
{}

/// Trait used for relaxing the lifetime requirements of the generator storage.
///
/// Implemented automatically for generators that accept any lifetime.
///
/// Direct usage of this trait is not considered part of SemVer.
pub unsafe trait RemitWithLifetime<T, O, X> {}

unsafe impl<T, O, F> RemitWithLifetime<T, O, ()> for F
    where
        F: for<'a> AsyncFnOnce<(Remit<'a, T, O>, )>,
{}

unsafe impl<T, O, X, F> RemitWithLifetime<T, O, (X, )> for F
    where
        F: for<'a> AsyncFnOnce<(X, Remit<'a, T, O>, )>,
{}

/// The storage used for iterators that poll a generator.
pub struct Generators<T, P, O = ()> {
    values: UnsafeCell<Values<T, O>>,
    future: Option<P>,
    _pin: PhantomPinned,
}

impl<T, P, O> Generators<T, P, O> {
    /// Provides the storage to be pinned when not using an allocation.
    pub fn new() -> Self {
        Generators {
            values: UnsafeCell::new(Values::Missing),
            future: None,
            _pin: PhantomPinned,
        }
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn pinned_exchange<'s, G>(
        self: Pin<&'s mut Self>,
        gen: G,
    ) -> Generator<'s, T, P, O>
        where
        // insures fn is not implemented only for 'static
            G: RemitWithLifetime<T, O, ()>,
        // insures P is properly defined, even if it actually has a lifetime
            G: FnOnce(Remit<'static, T, O>) -> P,
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
            _owner: None,
        }
    }

    #[allow(clippy::needless_lifetimes)]
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
            _owner: None,
        }
    }

    #[cfg(feature = "alloc")]
    pub fn boxed_exchange(gen: impl FnOnce(Remit<'static, T, O>) -> P) -> Generator<'static, T, P, O> {
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
            _owner: Some(rc),
        }
    }
}

impl<T, P, O: Default> Generators<T, P, O> {
    #[allow(clippy::needless_lifetimes)]
    /// Takes the pinned storage and the generator and provides an iterator.
    /// Stack based (does not use an allocation).
    ///
    /// The internal storage assumes the generator was valid for a provided `'static`,
    /// but requires the generator to be valid for all provided lifetimes.
    /// That is, the `Remit` provided to the generator cannot be moved out,
    /// even if at first glance it appears the storage does not have that restriction.
    /// In effect, this relaxes the lifetime requirements of the storage,
    /// but not the provided generator.
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
        self.pinned_exchange(gen).defaults()
    }

    #[allow(clippy::needless_lifetimes)]
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
        self.parameterized_exchange(gen, parameter).defaults()
    }

    #[cfg(feature = "alloc")]
    /// Uses an allocation so that the iterator does not need to be borrowed.
    /// Useful for returning an iterator from a function, where it can't be pinned to the stack.
    ///
    /// The generator only needs to be valid for `'static`; it does not need to be valid for all lifetimes.
    ///
    /// To pass in parameters, use a capturing closure.
    pub fn boxed(gen: impl FnOnce(Remit<'static, T, O>) -> P) -> GeneratorIterator<'static, T, P, impl Fn() -> O, O> {
        Self::boxed_exchange(gen).defaults()
    }
}

#[cfg(feature = "alloc")]
struct References<T, O> {
    interchange: UnsafeCell<Values<T, O>>,
    dropper: unsafe fn(*mut ()),
    checker: unsafe fn(*mut ()) -> bool,
    ptr: Cell<*mut ()>,
}

#[cfg(feature = "alloc")]
impl<T, O> References<T, O> {
    fn new<P>() -> Self {
        References {
            interchange: UnsafeCell::new(Values::Missing),
            dropper: Cycler::<P, T, O>::do_inner_drop,
            checker: Cycler::<P, T, O>::is_strong,
            // Note that `null_mut` is only until the surrounding Rc gets created.
            ptr: Cell::new(null_mut()),
        }
    }

    unsafe fn strong(this: *const Self) -> bool {
        let inner_ptr = (*addr_of!((*this).ptr)).get();
        // SOUND: checker is not pub, nor was inner_ptr,
        // thus still valid from instantiation
        //
        // SOUND: unsafe-fn, see Cycler::is_strong
        (*addr_of!((*this).checker))(inner_ptr)
    }

    unsafe fn dropping(this: *const Self) {
        let inner_ptr = (*addr_of!((*this).ptr)).get();
        // SOUND: dropper is not pub, nor was inner_ptr,
        // thus still valid from instantiation
        //
        // SOUND: dropper only called once for inner_ptr,
        // as inner_ptr only exists in this struct,
        // and dropping is only called once.
        //
        // SOUND: unsafe-fn, see Cycler::do_inner_drop
        (*addr_of!((*this).dropper))(inner_ptr)
    }
}

#[cfg(feature = "alloc")]
struct Cycler<P, T, O> {
    future: UnsafeCell<Option<P>>,
    references: References<T, O>,
    weak_inner: UnsafeCell<MaybeUninit<Weak<Cycler<P, T, O>>>>,
    _pin: PhantomPinned,
}

#[cfg(feature = "alloc")]
impl<P, T, O> Cycler<P, T, O> {
    #[inline(always)]
    /// Exclusive-ref must not reused.
    /// Resulting ptr must be kept !Send !Sync
    // NEED: erasing Cycler's storage generic, which ends up recursive
    unsafe fn ptr_convert(ptr: &mut Weak<Cycler<P, T, O>>) -> *mut () {
        ptr as *mut _ as _
    }

    /// ptr must be created with this Cycler's ptr_convert.
    /// May only be called once.
    // NEED: erasing Cycler's storage generic, which ends up recursive
    unsafe fn do_inner_drop(ptr: *mut ()) {
        let ptr: *mut Weak<Cycler<P, T, O>> = ptr as _;
        // SOUND: (Rc-race-condition) ptr_convert requires !Send !Sync
        // SOUND: (valid-ptr) ptr_convert instantiation
        // SOUND: (double-drop) can only be called once
        let _: Weak<Cycler<P, T, O>> = read(ptr);
    }

    /// ptr must be created with this Cycler's ptr_convert.
    /// Must not be called after do_inner_drop.
    // NEED: erasing Cycler's storage generic, which ends up recursive
    unsafe fn is_strong(ptr: *mut ()) -> bool {
        let ptr: *const Weak<Cycler<P, T, O>> = ptr as _;
        // SOUND: (use-after-free) can't be called after do_inner_drop
        // SOUND: (valid-ptr) ptr_convert instantiation
        // SOUND: (no exclusive ref violation) only exclusive-ref is do_inner_drop
        (*ptr).strong_count() > 0
    }
}

/// An iterator over generated values.
///
/// If the provided function `await`s without having remitting a value, the iterator will return `None`.
/// The iterator can continue to provide more values even after having returned `None` if more values are remitted during another poll.
/// If one or more values are available, it will not poll until they have been consumed.
///
/// The upper-bound of `size_hint` will be `None` iff the future has not completed.
pub struct Generator<'a, T, P, O = ()> {
    done: bool,
    mode: Mode<'a, T, O>,
    future: *mut P,
    #[cfg(feature = "alloc")]
    _owner: Option<Rc<Cycler<P, T, O>>>,
}

impl<'a, T, P, O> Generator<'a, T, P, O> {
    pub fn provider<F: FnMut() -> O>(self, provider: F) -> GeneratorIterator<'a, T, P, F, O> {
        GeneratorIterator {
            generator: self,
            provider,
        }
    }
}

impl<'a, T, P, O: Default> Generator<'a, T, P, O> {
    pub fn defaults(self) -> GeneratorIterator<'a, T, P, impl Fn() -> O, O> {
        GeneratorIterator {
            generator: self,
            provider: Default::default,
        }
    }
}

pub struct GeneratorIterator<'a, T, P, F, O = ()> {
    generator: Generator<'a, T, P, O>,
    provider: F,
}

impl<'s, T, P: Future<Output=()>, O: 's> Iterator for Generator<'s, T, P, O> {
    type Item = Exchange<'s, T, O>;
    fn next(&mut self) -> Option<Exchange<'s, T, O>> {
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

impl<'s, T, P: Future<Output=()>, O: 's> Generator<'s, T, P, O> {
    fn make_exchange(&mut self, entry: (T, *mut Option<O>)) -> Exchange<'s, T, O> {
        let (value, passback) = entry;
        let (indirection, indirection_ctx) = match self.mode {
            Mode::Pinned { value, .. } =>
                (
                    RemitBack::<O>::indirection_stack::<T> as Indirection<'s, O>,
                    value as *const (),
                ),
            #[cfg(feature = "alloc")]
            Mode::Boxed(references) => {
                let _ = Rc::downgrade(unsafe { self._owner.as_ref().unwrap_unchecked() }).into_raw();
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
}

#[must_use]
pub struct Exchange<'a, T, O> {
    value: T,
    passback: RemitBack<'a, O>,
}

impl<T, O> AsRef<T> for Exchange<'_, T, O> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<T, O> AsMut<T> for Exchange<'_, T, O> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<'a, T, O> Exchange<'a, T, O> {
    pub fn handle(self, func: impl FnOnce(T) -> O) {
        let (value, passback) = self.take();
        passback.provide(func(value));
    }

    pub fn provide(self, value: O) -> T {
        let Exchange {
            value: ret,
            passback,
        } = self;
        passback.provide(value);
        ret
    }

    pub fn take(self) -> (T, RemitBack<'a, O>) {
        let Exchange {
            value,
            passback,
        } = self;
        (value, passback)
    }
}

type Indirection<'a, O> = unsafe fn(&RemitBack<'a, O>) -> bool;

#[must_use]
pub struct RemitBack<'a, O> {
    indirection: Indirection<'a, O>,
    indirection_ctx: *const (),
    data: *mut Option<O>,
    _ref: PhantomData<&'a ()>,
}

impl<O> RemitBack<'_, O> {
    /// Must only be called once.
    unsafe fn check(&self) -> bool {
        // SOUND: only called once, see indirection
        (self.indirection)(self)
    }

    /// May only be called after check returns true.
    unsafe fn write(&self, value: O) {
        // SOUND: check() insured that RemitFuture hadn't been dropped
        write(self.data, Some(value))
    }

    fn remove<T>(&self, values: &mut Values<T, O>) -> bool {
        values.remove(self.data)
    }

    unsafe fn indirection_stack<T>(&self) -> bool {
        let values = &mut *(self.indirection_ctx as *mut Values<T, O>);
        self.remove(values)
    }

    #[cfg(feature = "alloc")]
    unsafe fn indirection_boxed<T>(&self) -> bool {
        let references: *const References<T, O> = self.indirection_ctx as _;
        if !References::strong(references) {
            References::dropping(references);
            return false;
        }
        References::dropping(references);
        let values = &mut *(*references).interchange.get();
        self.remove(values)
    }
}

impl<O> Drop for RemitBack<'_, O> {
    fn drop(&mut self) {
        // SOUND: `drop` only called once,
        // and other calls use ManuallyDrop.
        let _ = unsafe { self.check() };
    }
}

impl<O> RemitBack<'_, O> {
    pub fn provide(self, value: O) {
        let this = ManuallyDrop::new(self);
        unsafe {
            // SOUND: wont call check again due to skipping drop via ManuallyDrop
            if this.check() {
                // SOUND: check returned true
                this.write(value);
            }
        }
    }
}

impl<O: Default> RemitBack<'_, O> {
    #[inline(always)]
    pub fn provide_default(self) {
        self.provide(O::default());
    }
}

enum Values<T, O> {
    Present(T, *mut Option<O>),
    Waiting(*mut Option<O>),
    Missing,
    #[cfg(feature = "alloc")]
    Multiple(VecDeque<(Option<T>, *mut Option<O>)>),
}

impl<T, O> Values<T, O> {
    fn remove(&mut self, original_ptr: *mut Option<O>) -> bool {
        use Values::*;
        match self {
            Present(_, _)
            | Missing
                => false,
            Waiting(ptr) => {
                let ptr = *ptr;
                if eq(ptr, original_ptr) {
                    *self = Missing;
                    true
                } else {
                    false
                }
            },
            #[cfg(feature = "alloc")]
            Multiple(values) => {
                for (ix, &(ref provided, passback)) in values.iter().enumerate() {
                    if provided.is_some() {
                        continue
                    }
                    if eq(passback, original_ptr) {
                        // No-recursive drop because provided-is-none.
                        // No-panic because enumerate-ix
                        values.remove(ix);
                        return true;
                    }
                }
                false
            },
        }
    }

    fn next_inner(&mut self) -> Option<(T, *mut Option<O>)> {
        use Values::*;
        match self {
            Missing
            | Waiting(_)
            => None,
            &mut Present(_, passback) => {
                let Present(value, passback) = mem::replace(self, Waiting(passback))
                    else {
                        // SOUND: note exclusive-reference and surrounding match
                        unsafe { unreachable_unchecked() }
                    };
                Some((value, passback))
            },
            #[cfg(feature = "alloc")]
            Multiple(list) => {
                for (value, passback) in list.iter_mut() {
                    if let Some(value) = value.take() {
                        return Some((value, *passback));
                    }
                }
                None
            },
        }
    }

    #[cfg(feature = "alloc")]
    fn push_inner(&mut self, value: T, passback: *mut Option<O>) {
        use Values::*;
        let list = match self {
            Missing => {
                let Missing = mem::replace(self, Present(value, passback))
                    else {
                        // SOUND: note exclusive-reference and surrounding match
                        unsafe { unreachable_unchecked() };
                    };
                return;
            },
            &mut Waiting(old_passback) => {
                let Waiting(_) = mem::replace(self, Multiple(VecDeque::with_capacity(2)))
                    else {
                        // SOUND: note exclusive-reference and surrounding match
                        unsafe { unreachable_unchecked() };
                    };
                let Multiple(list) = self
                    else {
                        // SOUND: note assignment above
                        unsafe { unreachable_unchecked() };
                    };
                list.push_back((None, old_passback));
                list
            },
            Present(_, _) => {
                let Present(old_value, old_passback) = mem::replace(self, Multiple(VecDeque::with_capacity(2)))
                    else {
                        // SOUND: note exclusive-reference and surrounding match
                        unsafe { unreachable_unchecked() };
                    };
                let Multiple(list) = self
                    else {
                        // SOUND: note assignment above
                        unsafe { unreachable_unchecked() };
                    };
                list.push_back((Some(old_value), old_passback));
                list
            },
            Multiple(list) => list,
        };
        list.push_back((Some(value), passback));
    }

    #[cfg(not(feature = "alloc"))]
    fn push_inner(&mut self, value: T, ptr: *mut Option<O>) -> Values<T, O> {
        mem::replace(self, Values::Present(value, ptr))
    }
}

enum Mode<'a, T, O> {
    Pinned {
        value: *mut Values<T, O>,
        _lifetime: PhantomData<&'a ()>,
    },
    #[cfg(feature = "alloc")]
    Boxed(*const References<T, O>),
}

impl<T, O> Clone for Mode<'_, T, O> {
    fn clone(&self) -> Self {
        *self
    }

    fn clone_from(&mut self, source: &Self) {
        *self = *source
    }
}

impl<T, O> Copy for Mode<'_, T, O> {}

impl<T, O> Mode<'_, T, O> {
    #[inline(always)]
    fn values(&self) -> *mut Values<T, O> {
        match *self {
            Mode::Pinned {
                value,
                ..
            } => value,
            #[cfg(feature = "alloc")]
            // SOUND: (valid-ptr) Not-pub, and is always valid at instantiation.
            //
            // SOUND: (use-after-free) Not public type. Encapsulating type owns it.
            //
            // SOUND: (no exclusive ref violation)
            // * `*const ptr` never borrowed exclusively
            // * ptr never leaked
            //
            // NEED: erasing Cycler's storage generic, which ends up recursive
            Mode::Boxed(ptr) => unsafe { &*addr_of!((*ptr).interchange) }.get()
        }
    }

    #[inline(always)]
    fn next(&self) -> Option<(T, *mut Option<O>)> {
        // SOUND: (valid-ptr) Not-pub, and is always valid at instantiation.
        //
        // SOUND: (use-after-free) Not public type.
        // Either encapsulating type owns it, or reflected in lifetime.
        //
        // SOUND: (&mut exclusive)
        // * only accessed in this impl
        // * non-recursively (note: no calls to drop)
        // * behind UnsafeCell
        // * !Send, !Sync
        //
        // NEED: lock-free exchange
        // NEED: pinned-variant's lifetime cheat
        unsafe { &mut *self.values() }.next_inner()
    }

    #[inline(always)]
    fn push(&self, value: T, passback: *mut Option<O>) {
        // SOUND: (valid-ptr) Not-pub, and is always valid at instantiation.
        //
        // SOUND: (use-after-free) Not public type.
        // Either encapsulating type owns it, or reflected in lifetime.
        //
        // SOUND: (&mut exclusive)
        // * only accessed in this impl
        // * non-recursively (note: drop is after exclusive-reference is gone for no-alloc)
        // * behind UnsafeCell
        // * !Send, !Sync
        //
        // NEED: lock-free exchange
        // NEED: pinned-variant's lifetime cheat
        let _ = unsafe { &mut *self.values() }.push_inner(value, passback);
    }

    #[cfg(feature = "alloc")]
    /// Requires the box-ptr to be instantiated correctly.
    /// May not be called after dropping.
    // SOUND: (use-after-free) cannot be called after dropping()
    //
    // SOUND: (no exclusive ref violation)
    // * `*const ptr`s never borrowed exclusively
    // * ptrs never leaked
    //
    // NEED: erasing Cycler's storage generic, which ends up recursive
    // NEED: use-after-free prevention of value-exchange
    unsafe fn strong(&self) -> bool {
        if let &Mode::Boxed(ptr) = self {
            References::strong(ptr)
        } else {
            true
        }
    }

    #[cfg(not(feature = "alloc"))]
    const unsafe fn strong(&self) -> bool {
        true
    }
}

enum ExchangeState<T, O> {
    Waiting(T),
    // cannot make a finished state, because pinning
    Provided(UnsafeCell<Option<O>>, PhantomPinned),
}

struct RemitFuture<'a, T, O> {
    exchange: ExchangeState<T, O>,
    mode: Mode<'a, T, O>,
    _pin: PhantomPinned,
}

impl<T, O> Future for RemitFuture<'_, T, O> {
    type Output = O;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        if let ExchangeState::Provided(provided, _) = &this.exchange {
            if let Some(value) = unsafe { &mut *provided.get() }.take() {
                return Poll::Ready(value);
            }
            return Poll::Pending;
        }
        let ExchangeState::Waiting(value) = mem::replace(
            &mut this.exchange,
            ExchangeState::Provided(UnsafeCell::new(None), PhantomPinned)
        )
            else {
                unsafe { unreachable_unchecked() };
            };
        let ExchangeState::Provided(cell, _) = &this.exchange
            else {
                unsafe { unreachable_unchecked() }
            };
        let ptr = cell.get();
        if unsafe { this.mode.strong() } {
            this.mode.push(value, ptr);
        }

        Poll::Pending
    }
}

impl<T, O> Drop for RemitFuture<'_, T, O> {
    fn drop(&mut self) {
        let ExchangeState::Provided(cell, _) = &self.exchange
            else {
                return
            };
        let ptr = cell.get();
        if unsafe { &*ptr }.is_some() {
            return
        }
        unsafe { &mut *self.mode.values() }.remove(ptr);
    }
}

/// Allows a generator to provide values to an iterator.
/// A generator that only accepts the `'static` lifetime can only be used when boxed.
pub struct Remit<'a, T, O = ()>(Mode<'a, T, O>);

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
        Self::value_impl(self.0, value)
    }

    fn value_impl(mode: Mode<'_, T, O>, value: T) -> RemitFuture<'_, T, O> {
        RemitFuture {
            exchange: ExchangeState::Waiting(value),
            mode,
            _pin: Default::default(),
        }
    }

    #[cfg(feature = "alloc")]
    /// Requires the box-ptr to be instantiated correctly,
    /// and may only be called once.

    // SOUND: (use-after-free) free occurs here, and not read after
    //
    // SOUND: (no exclusive ref violation)
    // * `*const ptr`s never borrowed exclusively
    // * ptrs never leaked
    //
    // NEED: erasing Cycler's storage generic, which ends up recursive
    unsafe fn dropping(&mut self) {
        if let &mut Remit(Mode::Boxed(ptr)) = self {
            References::dropping(ptr)
        }
    }
}

#[cfg(feature = "alloc")]
impl<T, O> Drop for Remit<'_, T, O> {
    fn drop(&mut self) {
        // SOUND: Valid at instantiation
        // SOUND: Only call-site of dropping, and inner ptrs
        unsafe { self.dropping() }
    }
}
