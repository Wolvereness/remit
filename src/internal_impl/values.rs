use core::{
    hint::unreachable_unchecked,
    ptr::eq,
    mem,
};

#[cfg(feature = "alloc")]
use alloc::collections::VecDeque;

pub enum Values<T, O> {
    Present(T, *mut Option<O>),
    Waiting(*mut Option<O>),
    Missing,
    #[cfg(feature = "alloc")]
    Multiple(VecDeque<(Option<T>, *mut Option<O>)>),
}

impl<T, O> Values<T, O> {
    // SOUND: This function does not drop any T, preventing recursive exclusive references.
    pub(crate) fn remove(&mut self, original_ptr: *mut Option<O>) -> (Option<T>, bool) {
        use Values::*;
        match self {
            Missing
                => (None, false),
            Present(_, ptr) => {
                let ptr = *ptr;
                if eq(ptr, original_ptr) {
                    let Present(value, _) = mem::replace(self,Missing)
                        else {
                            // SOUND: note exclusive-reference and surrounding match
                            unsafe { unreachable_unchecked() }
                        };
                    (Some(value), true)
                } else {
                    (None, false)
                }
            },
            Waiting(ptr) => {
                let ptr = *ptr;
                (None, if eq(ptr, original_ptr) {
                    *self = Missing;
                    true
                } else {
                    false
                })
            },
            #[cfg(feature = "alloc")]
            Multiple(values) => {
                for (ix, &(_, passback)) in values.iter().enumerate() {
                    if eq(passback, original_ptr) {
                        // No-panic because enumerate-ix
                        return (values.remove(ix).and_then(|(value, _)| value), true);
                    }
                }
                (None, false)
            },
        }
    }

    pub(crate) fn next_inner(&mut self) -> Option<(T, *mut Option<O>)> {
        use Values::*;
        match self {
            Missing
            | Waiting(_)
                => None,
            &mut Present(_, passback) => {
                let Present(value, passback) = mem::replace(self, Waiting(passback))
                    else {
                        // SOUND: note exclusive-reference and surrounding match
                        unsafe { unreachable_unchecked() }
                    };
                Some((value, passback))
            },
            #[cfg(feature = "alloc")]
            Multiple(list) => {
                for (value, passback) in list.iter_mut() {
                    if let Some(value) = value.take() {
                        return Some((value, *passback));
                    }
                }
                None
            },
        }
    }

    #[cfg(feature = "alloc")]
    pub(crate) fn push_inner(&mut self, value: T, passback: *mut Option<O>) {
        use Values::*;
        let list = match self {
            Missing => {
                let Missing = mem::replace(self, Present(value, passback))
                    else {
                        // SOUND: note exclusive-reference and surrounding match
                        unsafe { unreachable_unchecked() };
                    };
                return;
            },
            &mut Waiting(old_passback) => {
                let Waiting(_) = mem::replace(self, Multiple(VecDeque::with_capacity(2)))
                    else {
                        // SOUND: note exclusive-reference and surrounding match
                        unsafe { unreachable_unchecked() };
                    };
                let Multiple(list) = self
                    else {
                        // SOUND: note assignment above
                        unsafe { unreachable_unchecked() };
                    };
                list.push_back((None, old_passback));
                list
            },
            Present(_, _) => {
                let Present(old_value, old_passback) = mem::replace(self, Multiple(VecDeque::with_capacity(2)))
                    else {
                        // SOUND: note exclusive-reference and surrounding match
                        unsafe { unreachable_unchecked() };
                    };
                let Multiple(list) = self
                    else {
                        // SOUND: note assignment above
                        unsafe { unreachable_unchecked() };
                    };
                list.push_back((Some(old_value), old_passback));
                list
            },
            Multiple(list) => list,
        };
        list.push_back((Some(value), passback));
    }

    #[cfg(not(feature = "alloc"))]
    pub(crate) fn push_inner(&mut self, value: T, ptr: *mut Option<O>) -> Values<T, O> {
        mem::replace(self, Values::Present(value, ptr))
    }
}
