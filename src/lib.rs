//! This crate is a helper utility for structs that need to drop using self instead of
//! &mut self as provided by [core::ops::Drop].
//!
//! This crate contains 2 things:
//!
//! 1. The trait [ValueDrop]. Types that need to drop using self should implement this trait.
//!
//! 2. The struct [AutoValueDrop]. This struct will automatically call [ValueDrop.drop] on it's
//! contents when this struct is dropped by normal Rusty means. It implements [core::ops::Deref]
//! and [core::ops::DerefMut] so it should be possible to use this as if it's the normal struct.
//! It also implements, [core::fmt::Debug], [core::clone::Clone], [core::default::Default],
//! [core::cmp::Eq], [core::cmp::PartialEq], [core::cmp::Ord], [core::cmp::PartialOrd], and
//! [core::hash::Hash] when possible.
//!
//! This crate is no_std by default.
//!
//! # Example
//!
//! ```rust,no_run
//! use c_crate_sys::{CData, init_c_data, free_c_data};
//! use selfdrop::{AutoValueDrop, ValueDrop};
//!
//! struct CWrapper {
//!     data: CData,
//!     argument: usize
//! }
//!
//! impl ValueDrop for CWrapper {
//!     fn drop(self) {
//!         //free_c_data's definition is fn free_c_data(data: CData, argument: usize);
//!         //As such, you cannot free this data from the normal Drop
//!         //without wrapping it in an Option or similar
//!         free_c_data(self.data, self.argument);
//!     }
//! }
//!
//! impl CWrapper {
//!     pub fn new(argument: usize) -> AutoValueDrop<CWrapper> {
//!         let data: CData = init_c_data(argument);
//!         let wrapper: CWrapper = CWrapper {data, argument};
//!         AutoValueDrop::new(wrapper)
//!     }
//! }
//! ```
#![no_std]

use core::mem::{forget, swap, ManuallyDrop};

use core::mem::uninitialized;
use core::ops::{Deref, DerefMut};

///A Drop implementation for types that need to use self instead of &mut self when dropping
pub trait ValueDrop {
    fn drop(self);
}

///A wrapper type that will automatically call [ValueDrop::drop] on it's contents when
/// this struct is dropped.
/// Realistically, types that are used in an AutoValueDrop should NOT implement [core::ops::Drop]
/// but at the moment, without negative trait bounds, mutually exclusive traits or similar,
/// I see no way to enforce this.
#[derive(Debug, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct AutoValueDrop<T: ValueDrop>(ManuallyDrop<T>);

impl<T: ValueDrop> AutoValueDrop<T> {
    ///Constructs a new [AutoValueDrop] value
    #[inline(always)]
    pub fn new(val: T) -> Self {
        Self(ManuallyDrop::new(val))
    }

    ///Get's the [AutoValueDrop]'s value. The inner data will NOT be automatically dropped.
    /// As such, if you call this method you should either put it back into an [AutoValueDrop] or
    /// ensure that you manually call drop on it.
    #[inline(always)]
    pub fn into_inner(mut slot: Self) -> T {
        //Can't just take because Self implements Drop
        let mut val = unsafe { uninitialized() };
        swap(&mut slot.0, &mut val);
        //Run forget on slot as it now contains uninitialized data
        forget(slot);
        ManuallyDrop::into_inner(val)
    }
}

impl<T: ValueDrop> Drop for AutoValueDrop<T> {
    #[inline(always)]
    fn drop(&mut self) {
        let mut val = unsafe { uninitialized() };
        swap(&mut self.0, &mut val);
        ManuallyDrop::into_inner(val).drop()
    }
}

impl<T: ValueDrop> Deref for AutoValueDrop<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ValueDrop> DerefMut for AutoValueDrop<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

unsafe impl<T: ValueDrop + Send> Send for AutoValueDrop<T> {}
unsafe impl<T: ValueDrop + Sync> Sync for AutoValueDrop<T> {}

#[cfg(test)]
mod tests {
    use crate::{AutoValueDrop, ValueDrop};

    #[test]
    fn basic_drop() {
        const X_VAL: usize = 5;

        struct DropTest {
            x: usize,
        }
        impl ValueDrop for DropTest {
            fn drop(self) {
                assert_eq!(self.x, X_VAL, "Basic Drop value was not what was expected");
            }
        }

        let x = DropTest { x: X_VAL };
        let _a = AutoValueDrop::new(x);
    }

    #[test]
    fn into_inner_no_drop() {
        const X_VAL: usize = 5;

        struct DropTest {
            _x: usize,
        }

        impl ValueDrop for DropTest {
            fn drop(self) {
                panic!("This drop should NOT be called");
            }
        }

        let x = DropTest { _x: X_VAL };
        let a = AutoValueDrop::new(x);
        let _y = AutoValueDrop::into_inner(a);
    }

    #[test]
    fn expected_value() {
        const X_VAL: usize = 5;

        struct DropTest {
            x: usize,
        }

        impl ValueDrop for DropTest {
            fn drop(self) {
                panic!("This drop should NOT be called");
            }
        }

        let x = DropTest { x: X_VAL };
        let a = AutoValueDrop::new(x);
        let y = AutoValueDrop::into_inner(a);
        assert_eq!(y.x, X_VAL, "Value was not what was expected");
    }

    #[test]
    fn expected_drop_count() {
        static mut DROP_COUNT: usize = 0;
        struct DropTest {
            _x: usize,
        }

        impl ValueDrop for DropTest {
            fn drop(self) {
                unsafe { DROP_COUNT += 1 }
            }
        }

        let x = AutoValueDrop::new(DropTest { _x: 5 });
        let y = AutoValueDrop::new(DropTest { _x: 6 });
        let z = AutoValueDrop::new(DropTest { _x: 7 });
        {
            let _auto_drops = x;
            let _auto_drops = y;
            let _auto_drops = z;
        }
        assert_eq!(
            unsafe { DROP_COUNT },
            3,
            "Dropped the wrong amount of times"
        );
    }

    #[test]
    fn clone_drop_count() {
        static mut LAST_DROP: usize = 0;
        #[derive(Copy, Clone)]
        struct DropTest {
            x: usize,
        }

        impl ValueDrop for DropTest {
            fn drop(self) {
                unsafe { LAST_DROP = self.x }
            }
        }

        let x = AutoValueDrop::new(DropTest { x: 5 });
        let mut y = x.clone();
        y.x = 10;
        drop(y);
        assert_eq!(unsafe { LAST_DROP }, 10, "Dropped in wrong order");
        drop(x);
        assert_eq!(unsafe { LAST_DROP }, 5, "Dropped in wrong order");
    }

}
