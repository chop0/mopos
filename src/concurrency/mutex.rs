use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use crate::concurrency::semaphore::{Semaphore, SemaphoreGuard};

pub struct Mutex<T> {
   pub semaphore: Semaphore,
    datum: UnsafeCell<T>,
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Mutex::new(T::default())
    }
}

unsafe impl<T: Send> Sync for Mutex<T> {}
unsafe impl<T: Send> Send for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(value: T) -> Self {
        Self {
            semaphore: Semaphore::with_permits(1),
            datum: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        let guard = self.semaphore.acquire(1);

        MutexGuard {
            inner: guard,
            reference: unsafe { &mut *self.datum.get() },
        }
    }

    pub unsafe fn force_unlock(&self) {
        self.semaphore.release(1);
    }

    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        Some(MutexGuard {
            inner: self.semaphore.try_acquire(1)?,
            reference: unsafe { &mut *self.datum.get() },
        })
    }
}

pub struct MutexGuard<'a, T> {
    inner: SemaphoreGuard<'a>,
    reference: &'a mut T,
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.reference
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.reference
    }
}