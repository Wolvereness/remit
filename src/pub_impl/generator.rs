use core::future::Future;

use super::super::{
    Exchange,
    Generator,
    GeneratorIterator,
};

impl<'a, T, P, O> Generator<'a, T, P, O> {
    /// Transforms into a [`GeneratorIterator`].
    ///
    /// Will use the provided generator to send values back through the [`Exchange`]s.
    pub fn provider<F: FnMut() -> O>(self, provider: F) -> GeneratorIterator<'a, T, P, F, O> {
        GeneratorIterator {
            generator: self,
            provider,
        }
    }
}

impl<'a, T, P, O: Default> Generator<'a, T, P, O> {
    /// Transforms into a [`GeneratorIterator`].
    ///
    /// Will use [`Default`] values to send through the [`Exchange`]s.
    pub fn defaults(self) -> GeneratorIterator<'a, T, P, impl Fn() -> O, O> {
        GeneratorIterator {
            generator: self,
            provider: Default::default,
        }
    }
}

impl<'s, T, P: Future<Output=()>, O: 's> Iterator for Generator<'s, T, P, O> {
    type Item = Exchange<'s, T, O>;
    fn next(&mut self) -> Option<Exchange<'s, T, O>> {
        self.impl_next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, if !self.done { None } else { Some(self.mode.len_upper_bound()) })
    }
}
