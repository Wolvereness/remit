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
//! use remit::{Generator, Remit};
//!
//! async fn gen(remit: Remit<'_, usize>) {
//!     remit.value(42).await;
//!     // Does not need to be limited
//!     for i in 1.. {
//!         remit.value(i).await
//!     }
//! }
//! for item in pin!(Generator::new()).of(gen).take(10) {
//!     println!("{item}");
//!     // Prints 42, 1, 2, 3, 4, 5, 6, 7, 8, 9
//! }
//! assert_eq!(vec![42, 1, 2, 3], pin!(Generator::new()).of(gen).take(4).collect::<Vec<_>>());
//! /*
//! // Rust has trouble determining the lifetime
//! assert_eq!(
//!     vec![1],
//!     pin!(Generator::new())
//!         .of(|remit: Remit<'_, usize>| async move { remit.value(1).await; })
//!         .collect::<Vec<_>>(),
//! );
//! */
//! # #[cfg(feature = "alloc")]
//! assert_eq!(vec![42, 1], Generator::boxed(gen).take(2).collect::<Vec<_>>());
//! # #[cfg(feature = "alloc")]
//! fn iter() -> impl Iterator<Item=usize> {
//!     Generator::boxed(gen)
//! }
//! ```
//!
//! Parameterized usage.
//! ```
//! # use std::pin::pin;
//! # use remit::{Generator, Remit};
//! # use std::fmt;
//! # #[cfg(feature = "alloc")] {
//! async fn scream<D: fmt::Display>(iter: impl Iterator<Item=D>, remit: Remit<'_, String>) {
//!     for person in iter {
//!         remit.value(format!("{person} scream!")).await
//!     }
//!     remit.value("... for ice cream!".to_string());
//! }
//! let expected: Vec<String> = ["You scream!", "I scream!", "We all scream!", "... for ice cream!"].iter().map(ToString::to_string).collect();
//! assert_eq!(
//!     expected,
//!     pin!(Generator::new()).parameterized(scream, ["You", "I", "We all"].iter()).collect::<Vec<String>>(),
//! );
//! assert_eq!(
//!     expected,
//!     Generator::boxed(|remit| scream(["You", "I", "We all"].iter(), remit)).collect::<Vec<String>>(),
//! );
//! # }
//! ```
//!
//! Usage of a generator that only functions for `'static`.
//! ```
//! # use remit::{Generator, Remit};
//! # #[cfg(feature = "alloc")] {
//! async fn gen(remit: Remit<'static, usize>) {
//!     remit.value(2).await;
//!     remit.value(3).await;
//!     remit.value(5).await;
//!     remit.value(7).await;
//! }
//! for item in Generator::boxed(gen) {
//!     println!("{item}");
//! }
//! assert_eq!(vec![2, 3, 5, 7], Generator::boxed(gen).collect::<Vec<_>>());
//! assert_eq!(vec![1], Generator::boxed(|remit| async move { remit.value(1).await; }).collect::<Vec<_>>());
//!
//! fn iter() -> impl Iterator<Item=usize> {
//!     Generator::boxed(gen)
//! }
//! # }
//! ```
//!
//! Unorthodox usage of "eagerly" yielding values.
//! ```
//! # use std::pin::pin;
//! # use remit::{Generator, Remit};
//! // These implementations run successfully.
//! // However, they trigger creation of a buffer with alloc.
//! async fn no_await(remit: Remit<'_, usize>) {
//!     let _discard_future = remit.value(2);
//!     let _discard_future = remit.value(3);
//!     let _discard_future = remit.value(5);
//!     let _discard_future = remit.value(7);
//! }
//! assert_eq!(
//!     if cfg!(feature = "alloc") {
//!         vec![2, 3, 5, 7]
//!     } else {
//!         vec![7]
//!     },
//!     pin!(Generator::new()).of(no_await).collect::<Vec<_>>(),
//! );
//!
//! async fn delay_await(remit: Remit<'_, usize>) {
//!     let first_remit = remit.value(11);
//!     remit.value(13).await;
//!     // Will poll-ready as the latter call implies all values are consumed.
//!     // A join will also do the same.
//!     first_remit.await;
//!
//!     let _ = remit.value(17);
//!     let _ = remit.value(19);
//!     // Even though the future is done, the values were already sent.
//! }
//! assert_eq!(
//!     if cfg!(feature = "alloc") {
//!         vec![11, 13, 17, 19]
//!     } else {
//!         vec![13, 19]
//!     },
//!     pin!(Generator::new()).of(delay_await).collect::<Vec<_>>()
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
//! fn iter_explicit<'a>(data: &'a str) -> GeneratorIterator<'static, usize, impl std::future::Future<Output=()> + 'a> {
//!     Generator::boxed(|remit| gen_explicit(data, remit))
//! }
//!
//! fn iter_implicit(data: &str) -> GeneratorIterator<'static, usize, impl std::future::Future<Output=()> + '_> {
//!     Generator::boxed(|remit| gen_implicit(data, remit))
//! }
//!
//! for item in iter_explicit(&data) {
//!     dbg!(item);
//! }
//! for item in iter_implicit(&data) {
//!     dbg!(item);
//! }
//! for item in Generator::boxed(|remit| gen_explicit(&data, remit)) {
//!     dbg!(item);
//! }
//! for item in Generator::boxed(|remit| gen_implicit(&data, remit)) {
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
//! for item in pin!(Generator::new()).parameterized(gen_implicit, &data) {
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
//! for item in pin!(Generator::new()).parameterized(gen_explicit, &data) {
//!     dbg!(item);
//! }
//! */
//! ```
//!
//! Incorrect attempt of a stack-based generator.
//! ```compile_fail
//! # use std::pin::pin;
//! # use remit::{Generator, Remit};
//! /// Only accepts `'static`, so it needs to be boxed.
//! async fn gen(remit: Remit<'static, usize>) {
//!     remit.value(1).await;
//! }
//! // Fails to compile, because gen is only `'static` and pinning is for the stack.
//! for item in pin!(Generator::new()).of(gen) {
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
        poll_fn,
    },
    hint::unreachable_unchecked,
    marker::{
        PhantomData,
        PhantomPinned,
    },
    mem,
    pin::Pin,
    task::{
        Poll,
        Context,
        Waker,
    },
};

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use core::{
    cell::Cell,
    mem::MaybeUninit,
    ptr::{
        addr_of,
        read,
        null_mut,
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
pub unsafe trait RemitWithLifetime<T, X> {}

unsafe impl<T, F> RemitWithLifetime<T, ()> for F
    where
        F: for<'a> AsyncFnOnce<(Remit<'a, T>, )>,
{}

unsafe impl<T, X, F> RemitWithLifetime<T, (X, )> for F
    where
        F: for<'a> AsyncFnOnce<(X, Remit<'a, T>, )>,
{}

/// The storage used for iterators that poll a generator.
pub struct Generator<T, P> {
    values: UnsafeCell<Values<T>>,
    future: Option<P>,
    _pin: PhantomPinned,
}

impl<T, P> Generator<T, P> {
    /// Provides the storage to be pinned when not using an allocation.
    pub fn new() -> Self {
        Generator {
            values: UnsafeCell::new(Values::Missing),
            future: None,
            _pin: PhantomPinned,
        }
    }

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
    ) -> GeneratorIterator<'s, T, P>
        where
            // insures fn is not implemented only for 'static
            G: RemitWithLifetime<T, ()>,
            // insures P is properly defined, even if it actually has a lifetime
            G: FnOnce(Remit<'static, T>) -> P,
    {
        let inner = unsafe { self.get_unchecked_mut() };
        let value = inner.values.get();
        let mode = Mode::Pinned {
            value,
            // This becomes 'static, and the trait-guard is where the real protection is
            _lifetime: PhantomData,
        };
        let future = gen(Remit(mode));
        let future = inner.future.insert(future);
        GeneratorIterator {
            done: false,
            mode,
            future,
            #[cfg(feature = "alloc")]
            _owner: None,
        }
    }

    #[allow(clippy::needless_lifetimes)]
    /// The same as [`Generator::of()`] but allows passing a parameter in.
    pub fn parameterized<'s, G, X>(
        self: Pin<&'s mut Self>,
        gen: G,
        parameter: X,
    ) -> GeneratorIterator<'s, T, P>
        where
            // insures fn is not implemented only for 'static
            G: RemitWithLifetime<T, (X,)>,
            // insures P is properly defined, even if it actually has a lifetime
            G: FnOnce(X, Remit<'static, T>) -> P,
    {
        let inner = unsafe { self.get_unchecked_mut() };
        let value = inner.values.get();
        let mode = Mode::Pinned {
            value,
            // This becomes 'static, and the trait-guard is where the real protection is
            _lifetime: PhantomData,
        };
        let future = gen(parameter, Remit(mode));
        let future = inner.future.insert(future);
        GeneratorIterator {
            done: false,
            mode,
            future,
            #[cfg(feature = "alloc")]
            _owner: None,
        }
    }

    #[cfg(feature = "alloc")]
    /// Uses an allocation so that the iterator does not need to be borrowed.
    /// Useful for returning an iterator from a function, where it can't be pinned to the stack.
    ///
    /// The generator only needs to be valid for `'static`; it does not need to be valid for all lifetimes.
    ///
    /// To pass in parameters, use a capturing closure.
    pub fn boxed(gen: impl FnOnce(Remit<'static, T>) -> P) -> GeneratorIterator<'static, T, P> {
        let rc = Rc::new(Cycler {
            future: Default::default(),
            references: References::new::<P>(),
            weak_inner: UnsafeCell::new(MaybeUninit::uninit()),
            _pin: Default::default(),
        });
        let weak = Rc::downgrade(&rc);
        let ptr: *mut Weak<Cycler<P, T>> = unsafe { &mut *rc.weak_inner.get() }.write(weak);
        rc.references.ptr.set(ptr as _);

        let mode = Mode::Boxed(&rc.references);
        let future = unsafe { &mut *rc.future.get() }.insert(gen(Remit(mode)));

        GeneratorIterator {
            done: false,
            mode,
            future,
            _owner: Some(rc),
        }
    }
}

#[cfg(feature = "alloc")]
struct References<T> {
    interchange: UnsafeCell<Values<T>>,
    dropper: unsafe fn(*mut ()),
    checker: unsafe fn(*mut ()) -> bool,
    ptr: Cell<*mut ()>,
}

#[cfg(feature = "alloc")]
impl<T> References<T> {
    fn new<P>() -> Self {
        References {
            interchange: UnsafeCell::new(Values::Missing),
            dropper: Cycler::<P, T>::do_inner_drop,
            checker: Cycler::<P, T>::is_strong,
            ptr: Cell::new(null_mut()),
        }
    }
}

#[cfg(feature = "alloc")]
struct Cycler<P, T> {
    future: UnsafeCell<Option<P>>,
    references: References<T>,
    weak_inner: UnsafeCell<MaybeUninit<Weak<Cycler<P, T>>>>,
    _pin: PhantomPinned,
}

#[cfg(feature = "alloc")]
impl<P, T> Cycler<P, T> {
    unsafe fn do_inner_drop(ptr: *mut ()) {
        let ptr: *mut Weak<Cycler<P, T>> = ptr as _;
        let _: Weak<Cycler<P, T>> = read(ptr);
    }

    unsafe fn is_strong(ptr: *mut ()) -> bool {
        let ptr: *const Weak<Cycler<P, T>> = ptr as _;
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
pub struct GeneratorIterator<'a, T, P> {
    done: bool,
    mode: Mode<'a, T>,
    future: *mut P,
    #[cfg(feature = "alloc")]
    _owner: Option<Rc<Cycler<P, T>>>,
}

impl<T, P: Future<Output=()>> Iterator for GeneratorIterator<'_, T, P> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if let Some(value) = self.mode.next() {
            return Some(value)
        }
        if self.done {
            return None
        }
        // FIXME: https://github.com/rust-lang/rust/issues/102012
        let waker = unsafe { Waker::from_raw(context::NOOP_WAKER) };
        if let Poll::Ready(()) = unsafe { Pin::new_unchecked(&mut *self.future) }.poll(&mut Context::from_waker(&waker)) {
            self.done = true;
        }
        self.mode.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.mode.len();
        if self.done {
            (len, Some(len))
        } else {
            (len, None)
        }
    }
}

enum Values<T> {
    Present(T),
    Missing,
    #[cfg(feature = "alloc")]
    Multiple(VecDeque<T>),
}

enum Mode<'a, T> {
    Pinned {
        value: *mut Values<T>,
        _lifetime: PhantomData<&'a ()>,
    },
    #[cfg(feature = "alloc")]
    Boxed(*const References<T>),
}

impl<T> Clone for Mode<'_, T> {
    fn clone(&self) -> Self {
        *self
    }

    fn clone_from(&mut self, source: &Self) {
        *self = *source
    }
}

impl<T> Copy for Mode<'_, T> {}

impl<T> Mode<'_, T> {
    #[inline(always)]
    fn values(&self) -> *mut Values<T> {
        match *self {
            Mode::Pinned {
                value,
                ..
            } => value,
            #[cfg(feature = "alloc")]
            Mode::Boxed(ptr) => unsafe { &*addr_of!((*ptr).interchange) }.get()
        }
    }

    #[inline(always)]
    fn next(&self) -> Option<T> {
        Self::next_inner(unsafe { &mut *self.values() })
    }

    fn next_inner(values: &mut Values<T>) -> Option<T> {
        use Values::*;
        match values {
            Missing => None,
            Present(_) =>
                if let Present(value) = mem::replace(values, Missing) {
                    Some(value)
                } else { unsafe { unreachable_unchecked() } },
            #[cfg(feature = "alloc")]
            Multiple(list) => list.pop_front(),
        }
    }

    #[inline(always)]
    fn push(&self, value: T) {
        // Drop-call occurs after mutable reference is gone (for no-alloc).
        let _ = Self::push_inner(unsafe { &mut *self.values() }, value);
    }

    #[cfg(feature = "alloc")]
    fn push_inner(values: &mut Values<T>, value: T) {
        use Values::*;
        match values {
            Missing => *values = Present(value),
            Present(_) => {
                let Present(old) = mem::replace(values, Missing)
                    else { unsafe { unreachable_unchecked() } };
                let mut list = VecDeque::with_capacity(2);
                list.push_back(old);
                list.push_back(value);
                *values = Multiple(list);
            },
            Multiple(list) => list.push_back(value),
        }
    }

    #[cfg(not(feature = "alloc"))]
    fn push_inner(values: &mut Values<T>, value: T) -> Values<T> {
        mem::replace(values, Values::Present(value))
    }

    #[inline(always)]
    fn len(&self) -> usize {
        Self::len_inner(unsafe { &*self.values() })
    }

    fn len_inner(values: &Values<T>) -> usize {
        use Values::*;
        match values {
            Present(_) => 1,
            Missing => 0,
            #[cfg(feature = "alloc")]
            Multiple(list) => list.len(),
        }
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        Self::is_empty_inner(unsafe { &*self.values() })
    }

    fn is_empty_inner(values: &Values<T>) -> bool {
        use Values::*;
        match values {
            Present(_) => false,
            Missing => true,
            #[cfg(feature = "alloc")]
            Multiple(list) => list.is_empty(),
        }
    }
}

/// Allows a generator to provide values to an iterator.
/// A generator that only accepts the `'static` lifetime can only be used when boxed.
pub struct Remit<'a, T>(Mode<'a, T>);

impl<T> Remit<'_, T> {
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
    pub fn value(&self, value: T) -> impl Future<Output=()> + '_ {
        self.value_impl(value)
    }

    #[cfg(not(feature = "alloc"))]
    fn value_impl(&self, value: T) -> impl Future<Output=()> + '_ {
        self.0.push(value);
        poll_fn(|_ctx|
            if self.0.is_empty() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        )
    }

    #[cfg(feature = "alloc")]
    fn value_impl(&self, value: T) -> impl Future<Output=()> + '_ {
        if unsafe { self.strong() } {
            self.0.push(value);
        }
        poll_fn(|_ctx|
            if unsafe { self.strong() } && self.0.is_empty() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        )
    }

    #[cfg(feature = "alloc")]
    unsafe fn strong(&self) -> bool {
        if let &Remit(Mode::Boxed(ptr)) = self {
            let inner_ptr = (*addr_of!((*ptr).ptr)).get();
            (*addr_of!((*ptr).checker))(inner_ptr)
        } else {
            true
        }
    }

    #[cfg(feature = "alloc")]
    unsafe fn dropping(&mut self) {
        if let &mut Remit(Mode::Boxed(ptr)) = self {
            let inner_ptr = (*addr_of!((*ptr).ptr)).get();
            (*addr_of!((*ptr).dropper))(inner_ptr)
        }
    }
}

#[cfg(feature = "alloc")]
impl<T> Drop for Remit<'_, T> {
    fn drop(&mut self) {
        unsafe { self.dropping() }
    }
}
