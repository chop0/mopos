use alloc::sync::Arc;
use core::cell::RefCell;
use core::ops::Deref;
use core::sync::atomic::{AtomicPtr, Ordering};
use core::sync::atomic::Ordering::SeqCst;

use crate::Mutex;

pub struct RCU<T> {
    root: RefCell<Arc<T>>,
    datum: AtomicPtr<T>,
    swap_mutex: Mutex<()>
}

unsafe impl<T: Send + Sync> Send for RCU<T> {}
unsafe impl<T: Send + Sync> Sync for RCU<T> {}

impl<T: Default> Default for RCU<T> {
    fn default() -> Self {
        RCU::new(T::default())
    }
}

impl<T> RCU<T> {
    pub fn new(value: T) -> Self {
        let arc = Arc::new(value);
        Self {
            root: RefCell::new(arc.clone()),
            datum: AtomicPtr::new(Arc::as_ptr(&arc) as *mut _),
            swap_mutex: Mutex::new(())
        }
    }

    fn get_arc(&self) -> Arc<T> {
        unsafe {
            Arc::from_raw(self.datum.load(Ordering::SeqCst))
        }
    }

    pub fn read(&self) -> ReadGuard<T> {
        ReadGuard {
            datum: self.get_arc()
        }
    }

    pub fn update_and_get(&self, new_value: T) -> Arc<T> {
        let guard = self.swap_mutex.lock();
        let new_arc = Arc::new(new_value);
        self.datum.store(Arc::as_ptr(&new_arc) as *mut _, SeqCst);
        let result = self.root.replace(new_arc.clone());
        drop(guard);

        result
    }
}

pub struct ReadGuard<T> {
    datum: Arc<T>
}

impl<T> Deref for ReadGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.datum
    }
}