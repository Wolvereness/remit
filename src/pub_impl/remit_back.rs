use super::super::RemitBack;

impl<O> RemitBack<'_, O> {
    #[inline(always)]
    pub fn provide(self, value: O) {
        self.impl_provide(value)
    }
}

impl<O: Default> RemitBack<'_, O> {
    #[inline(always)]
    pub fn provide_default(self) {
        self.provide(O::default());
    }
}
