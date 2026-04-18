use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

// A simple spinlock that protects shared ressorces
//
// The spinlock uses an AtomicBool to keep track on the status of the lock. Is it taken or not.
// When you call "lock()" it spins (busy waits) until the lock is available
//
pub struct Spinlock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

// UnsafeCell is not Sync by default - so we tell the compeiler
// that our spinlock is safe to share between contexts (interrupts/thread)
// because we protect acces with atomics-
unsafe impl<T> Sync for Spinlock<T> {}

impl<T> Spinlock<T> {
    pub const fn new(value: T) -> Spinlock<T> {
        Spinlock {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }
}
