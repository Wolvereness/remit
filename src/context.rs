use std::{
    sync::{
        Arc,
        Once,
    },
    mem::MaybeUninit,
    task::{
        Context,
        Wake,
        Waker,
    },
};

struct WakeImpl;

impl Wake for WakeImpl {
    fn wake(self: Arc<Self>) {}
    fn wake_by_ref(self: &Arc<Self>) {}
}

static mut WAKER: MaybeUninit<Waker> = MaybeUninit::uninit();
static INIT: Once = Once::new();

unsafe fn init() {
    WAKER.write(Waker::from(Arc::new(WakeImpl)));
}

pub fn get() -> Context<'static> {
    INIT.call_once(|| unsafe { init() });

    unsafe { Context::from_waker(&WAKER.assume_init_ref()) }
}
