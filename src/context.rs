use core::task::{
    RawWaker,
    RawWakerVTable,
};

const fn clone_noop(ptr: *const ()) -> RawWaker {
    RawWaker::new(ptr, &NOOP_WAKER_V_TABLE)
}
const fn noop(_: *const ()) {}

const NOOP_WAKER_V_TABLE: RawWakerVTable = RawWakerVTable::new(
    clone_noop,
    noop,
    noop,
    noop,
);

pub const NOOP_WAKER: RawWaker = clone_noop(&NOOP_WAKER_V_TABLE as *const _ as _);
