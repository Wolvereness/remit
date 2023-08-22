use super::super::RemitBack;

impl<O> RemitBack<'_, O> {
    #[inline(always)]
    /// Sends the value back to the associated `Future` that was awaited.
    pub fn provide(self, value: O) {
        self.impl_provide(value)
    }
}

impl<O: Default> RemitBack<'_, O> {
    #[inline(always)]
    /// Sends a [default](Default) value back to the associated `Future` that was awaited.
    pub fn provide_default(self) {
        self.provide(O::default());
    }
}
