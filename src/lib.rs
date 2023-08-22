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
//! async fn health_regen(
//!     (starting, regen, minimum): (usize, usize, usize),
//!     remit: Remit<'_, usize, usize>,
//! ) {
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
//! # #[cfg(feature = "alloc")] {
//! buffer.clear();
//! for exchange in Generators::boxed_exchange(|remit| health_regen((375, 2, 30), remit)) {
//!     let (previous, to_damage) = exchange.take();
//!     buffer.push(previous);
//!     to_damage.provide(previous * 2 / 9);
//! }
//! assert_eq!(
//!     buffer,
//!     vec![375, 294, 231, 182, 144, 114, 91, 73, 59, 48, 40, 34],
//! );
//! # }
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
    marker::{
        PhantomData,
        PhantomPinned,
    },
};

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::rc::{
    Rc,
};

mod context;
mod internal_impl {
    //! Contributors must assume all internal parts need to be aware of all other internal parts
    //! for safety. The usage of unsafe is designed to facilitate:
    //!
    //! * **indirectly** self-referential structs
    //! * type erasure through reified function pointers with raw-pointer data
    //! * core functionality, like pinning and lock-free access (!Send !Sync)
    //!
    //! The lifetime of storage resembles 'static when boxed, via Rc. The Rc is held strongly in
    //! the Generator, but weakly in Remit. When Remit accesses the values, it needs to check that
    //! the Generator is still strongly held. The Remit could have been leaked outside of the
    //! future, so its drop is not inherently tied to the generator.
    //!
    //! The lifetime of storage is in a pinned Generators, and Generator will mutably borrow it.
    //! Generators is pinned so that it can store the Future and poll it. Re-use of Generators is
    //! valid usage of the API, which just does an Option::insert (dropping the old value
    //! in-place). Remit will need an arbitrarily short lifetime, and may be held inside the
    //! Future. This itself presents a concept that Values needs to be considered pinned as well,
    //! as that's what Remit is actually pointing at.
    //!
    //! RemitFuture will not send its value until it is polled, because polling insures it is
    //! pinned, such that RemitBack will be able to send a value back. Pinning also insures that
    //! either cleanup will properly occur or the location remains valid. RemitBack will check to
    //! see if the cleanup occurred before writing, and presence implies validity (per Pin).
    //!
    //! All usages of `&mut Values` must be short-lived. !Send, !Sync, and no recursion.

    // types inherently crate
    pub mod mode;
    pub mod remit;
    pub mod values;

    #[cfg(feature = "alloc")]
    pub mod references;
    #[cfg(feature = "alloc")]
    pub mod cycler;

    // types inherently pub
    mod generators;
    mod generator;
    mod remit_back;
}
mod pub_impl {
    //! Should not include any need of unsafe or special consideration by users.
    //! If it compiles against this API, it should be sound.

    mod remit;
    mod exchange;
    mod generators;
    mod fn_traits;
    mod remit_back;
    mod generator_iter;
    mod generator;
}

use internal_impl::{
    mode::Mode,
    values::Values,
};

#[cfg(feature = "alloc")]
use internal_impl::{
    references::References,
    cycler::Cycler,
};

/// Trait used for relaxing the lifetime requirements of the generator storage.
///
/// Implemented automatically for generators that accept any lifetime.
///
/// Direct usage of this trait is not considered part of SemVer.
pub unsafe trait RemitWithLifetime<T, O, X> {}

/// The storage used for iterators that poll a generator.
///
/// Stack-based generation requires pinning a [`Generators`],
/// while heap-based generation will internally handle the storage.
pub struct Generators<T, P, O = ()> {
    values: UnsafeCell<Values<T, O>>,
    future: Option<P>,
    _pin: PhantomPinned,
}

/// The container for a particular generator, iterable over [`Exchange`]s.
///
/// APIs that provide a [`GeneratorIterator`] are suggested when no meaningful data is sent back in [`Exchange`].
/// These include [`Generator::defaults()`], [`Generator::provider()`],
/// or most preferably the associated constructions from [`Generators`].
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
    owner: Option<Rc<Cycler<P, T, O>>>,
}

/// An iterator over only the generated values.
///
/// If the provided function `await`s without having remitting a value, the iterator will return `None`.
/// The iterator can continue to provide more values even after having returned `None` if more values are remitted during another poll.
/// If one or more values are available, it will not poll until they have been consumed.
///
/// The upper-bound of `size_hint` will be `None` iff the future has not completed.
pub struct GeneratorIterator<'a, T, P, F, O = ()> {
    generator: Generator<'a, T, P, O>,
    provider: F,
}

#[must_use]
/// Holds the incoming value and handles sending values back into the generator.
///
/// Values should always be provided, or the awaiting sites will never complete.
pub struct Exchange<'a, T, O> {
    value: T,
    passback: RemitBack<'a, O>,
}

type Indirection<'a, O> = unsafe fn(&RemitBack<'a, O>) -> bool;

#[must_use]
/// Handles sending values back into the generator.
///
/// Values should always be provided, or the awaiting sites will never complete.
pub struct RemitBack<'a, O> {
    indirection: Indirection<'a, O>,
    indirection_ctx: *const (),
    data: *mut Option<O>,
    _ref: PhantomData<&'a ()>,
}

/// Allows a generator to provide values to an iterator.
///
/// A generator that only accepts the `'static` lifetime can only be used when boxed.
pub struct Remit<'a, T, O = ()>(Mode<'a, T, O>);
