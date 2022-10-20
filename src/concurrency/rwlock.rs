use core::sync::atomic::{AtomicUsize, Ordering};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use crate::task::executor::yield_;

pub struct RwLock<T> {
    pending_or_locked_writers: AtomicUsize,
    holder_count: AtomicUsize,
    datum: UnsafeCell<T>,
}

impl<T> RwLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            pending_or_locked_writers: AtomicUsize::new(0),
            holder_count: AtomicUsize::new(0),
            datum: UnsafeCell::new(value),
        }
    }

    pub fn read(&self) -> ReadGuard<T> {
        loop {
            self.holder_count.fetch_add(1, Ordering::AcqRel);

            if self.pending_or_locked_writers.load(Ordering::Acquire) == 0 {
                return ReadGuard {
                    readers: &self.holder_count,
                    datum: unsafe { &*self.datum.get() },
                };
            }

            self.holder_count.fetch_sub(1, Ordering::AcqRel);
            yield_();
        }
    }

    pub fn write(&self) -> WriteGuard<T> {
        self.pending_or_locked_writers.fetch_add(1, Ordering::AcqRel);

        loop {
            'inner: loop {
                let holder_count = self.holder_count.load(Ordering::Acquire);
                if holder_count != 0 {
                    yield_();
                    continue 'inner;
                }

                if self.holder_count.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire).is_ok() {
                    return WriteGuard {
                        pending: &self.pending_or_locked_writers,
                        holders: &self.holder_count,
                        datum: unsafe { &mut *self.datum.get() },
                    };
                }
            }
        }
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> Self {
        RwLock::new(T::default())
    }
}

unsafe impl<T: Send> Sync for RwLock<T> {}

unsafe impl<T: Send> Send for RwLock<T> {}

pub struct ReadGuard<'a, T> {
    readers: &'a AtomicUsize,
    datum: &'a T,
}

impl<'a, T> Drop for ReadGuard<'a, T> {
    fn drop(&mut self) {
        self.readers.fetch_sub(1, Ordering::AcqRel);
    }
}

impl<'a, T> Deref for ReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &'a Self::Target {
        self.datum
    }
}

pub struct WriteGuard<'a, T> {
    pending: &'a AtomicUsize,
    holders: &'a AtomicUsize,
    datum: &'a mut T,
}

impl<'a, T> Drop for WriteGuard<'a, T> {
    fn drop(&mut self) {
        self.pending.fetch_sub(1, Ordering::AcqRel);
        self.holders.fetch_sub(1, Ordering::AcqRel);
    }
}

impl<'a, T> Deref for WriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> & Self::Target {
        self.datum
    }
}

impl<'a, T> DerefMut for WriteGuard<'a, T> {
    fn deref_mut(&mut self) -> & mut Self::Target {
        self.datum
    }
}
