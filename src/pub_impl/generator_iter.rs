use core::future::Future;

use super::super::{
    GeneratorIterator,
    Exchange,
};

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

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.generator.size_hint()
    }
}
