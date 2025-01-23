use std::cell::UnsafeCell;
use std::ops::Deref;

/// This struct is an "bypass" to avoid locks when
/// we are sure the type contained within will be used in only one thread
///
/// Implements `Send` and `Sync`.
///
/// Safety:
///
/// Only one mutable reference can exist.
pub struct MutCell<T> {
    data: UnsafeCell<T>,
}

impl<T> MutCell<T> {
    /// Creates a new MutCell.
    pub unsafe fn new(data: T) -> MutCell<T> {
        MutCell {
            data: UnsafeCell::new(data),
        }
    }

    /// Obtains a mutable reference to the type within.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }

    /// Obtains an immutable reference to the type within.
    pub fn get(&self) -> &T {
        unsafe { &*self.data.get() }
    }
}

impl<T> Deref for MutCell<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.data.get() }
    }
}

// Relatively unsafe.
// impl<T> DerefMut for MutCell<T> {
//     fn deref_mut(&mut self) -> &mut T {
//         self.data.get_mut()
//     }
// }

unsafe impl<T> Sync for MutCell<T> {}
unsafe impl<T> Send for MutCell<T> {}
