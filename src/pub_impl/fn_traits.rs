use core::future::Future;

use super::super::{
    RemitWithLifetime,
    Remit,
};

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

unsafe impl<T, O, F> RemitWithLifetime<T, O, ()> for F
    where
        F: for<'a> AsyncFnOnce<(Remit<'a, T, O>, )>,
{}

unsafe impl<T, O, X, F> RemitWithLifetime<T, O, (X, )> for F
    where
        F: for<'a> AsyncFnOnce<(X, Remit<'a, T, O>, )>,
{}
