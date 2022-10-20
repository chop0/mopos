use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crate::task::executor::yield_;

static FORCE_DISABLE: AtomicBool = AtomicBool::new(false);
pub struct Semaphore {
    permits: AtomicUsize,
}

impl Semaphore {
    pub const fn with_permits(count: usize) -> Self {
        Semaphore {
            permits: AtomicUsize::new(count),
        }
    }

    pub fn acquire(&self, desired_permits: usize) -> SemaphoreGuard {
        loop {
            let available_permits = self.permits.load(Ordering::Acquire);

            if available_permits >= desired_permits &&
                self.permits.compare_exchange(available_permits, available_permits - desired_permits, Ordering::AcqRel, Ordering::Acquire).is_ok()
            {
                return SemaphoreGuard(self, desired_permits);
            }

            // if available_permits == 0 && !x86_64::instructions::interrupts::are_enabled() {
            //     panic!("deadlock in Semaphore::acquire;  interrupts are disabled, but we don't have a permit");
            // }
        }
    }

    pub fn try_acquire(&self, desired_permits: usize) -> Option<SemaphoreGuard> {
        let available_permits = self.permits.load(Ordering::Acquire);

        if available_permits >= desired_permits &&
            self.permits.compare_exchange(available_permits, available_permits - desired_permits, Ordering::AcqRel, Ordering::Acquire).is_ok()
        {
            Some(SemaphoreGuard(self, desired_permits))
        } else {
            None
        }
    }

    pub fn acquire_unguarded(&self, desired_permits: usize) -> usize {
        loop {
            let available_permits = self.permits.load(Ordering::Acquire);

            if available_permits >= desired_permits &&
                self.permits.compare_exchange(available_permits, available_permits - desired_permits, Ordering::AcqRel, Ordering::Acquire).is_ok()
            {
                return available_permits - desired_permits;
            } else {
                if available_permits == 0 && !x86_64::instructions::interrupts::are_enabled() {
                    panic!("deadlock in Semaphore::acquire_unguarded;  interrupts are disabled, but we don't have a permit");
                }

                while self.permits.load(Ordering::Relaxed) == available_permits {
                    yield_();
                }
            }
        }
    }

    pub fn release(&self, count: usize) {
        self.permits.fetch_add(count, Ordering::Release);
    }
}

pub struct SemaphoreGuard<'a>(&'a Semaphore, usize);

impl<'a> Drop for SemaphoreGuard<'a> {
    fn drop(&mut self) {
        self.0.permits.fetch_add(self.1, Ordering::Release);
    }
}
