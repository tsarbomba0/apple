use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};

pub struct DummyMutex<T> {
    data: UnsafeCell<T>,
}

impl<T> DummyMutex<T> {
    pub fn new(data: T) -> DummyMutex<T> {
        DummyMutex {
            data: UnsafeCell::new(data),
        }
    }
    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}

impl<T> Deref for DummyMutex<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.data.get() }
    }
}

impl<T> DerefMut for DummyMutex<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }
}

unsafe impl<T> Sync for DummyMutex<T> {}
unsafe impl<T> Send for DummyMutex<T> {}
