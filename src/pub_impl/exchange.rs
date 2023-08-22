use super::super::{
    Exchange,
    RemitBack,
};

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
    /// The function will be used to create a value to send back.
    pub fn handle(self, func: impl FnOnce(T) -> O) {
        let (value, passback) = self.take();
        passback.provide(func(value));
    }

    /// Sends a value back.
    ///
    /// The underlying received value is returned.
    pub fn provide(self, value: O) -> T {
        let Exchange {
            value: ret,
            passback,
        } = self;
        passback.provide(value);
        ret
    }

    /// Take ownership of the underlying value.
    ///
    /// The [`RemitBack`] can send a value back.
    pub fn take(self) -> (T, RemitBack<'a, O>) {
        let Exchange {
            value,
            passback,
        } = self;
        (value, passback)
    }
}