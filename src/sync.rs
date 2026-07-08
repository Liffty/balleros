use core::arch::asm;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

// ============================================================
// Interrupt-helpers
// ============================================================

// Læs om interrupts er slået til lige nu.
// RFLAGS bit 9 (IF - Interrupt Flag) fortæller det: 1 = til, 0 = fra.
// pushfq skubber RFLAGS på stakken, vi popper den ind i en variable og masker bit 9
fn interrupts_enabled() -> bool {
    let flags: u64;
    unsafe {
        asm!("pushfq", "pop {}", out(reg) flags, options(nomem));
    }
    (flags & (1 << 9)) != 0
}

fn disable_interrupts() {
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
}

fn enable_interrupts() {
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
}

// ============================================================
// Spinlock<T> - grundlæggende gensidig udelukkelse
// ============================================================
// Besktytter data mod at flere tilgår det samtidigt
// "spin" = venter i en løkke til låsen er fri, i stedet fro at sove

pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

// Fortæller Rust det er sikkert at dele en spinlock mellem tråde
// Låsen sikre selv adgangen
unsafe impl<T> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub const fn new(data: T) -> SpinLock<T> {
        SpinLock {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    // Tag låsen. Retunerer en guard der giver adgang til data
    // og som frigibver låsen automatsik når den droppes.
    pub fn lock(&self) -> SpinGuard<T> {
        // Prøv at sætte låsen fra false til true. Spin indtil det lykkes.
        while self
        .locked
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
        {
            // Fortæl CPU'en vi spinner
            core::hint::spin_loop();
        }
        SpinGuard { lock: self}
    }
}

// RAII-guard: Så længde den lever, holver vi låsen.
// Deref/DerefMut giver adgang til de beskyttede data.
pub struct SpinGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<'a, T> core::ops::Deref for SpinGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> core::ops::DerefMut for SpinGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get()}
    }
}

impl<'a, T> Drop for SpinGuard<'a, T> {
    fn drop(&mut self) {
        // Frigiv låsen. Release parrer med Acquire i lock()
        self.lock.locked.store(false, Ordering::Release);
    }
}



// ============================================================
// IrqSafeSpinLock<T> - lås der er sikker at tage i interrupts-handler
// ============================================================
// Undgår deadlock i interrupts

pub struct IrqSafeSpinLock<T> {
    inner: SpinLock<T>,
}

unsafe impl<T> Sync for IrqSafeSpinLock<T> {}

impl<T> IrqSafeSpinLock<T> {
    pub const fn new(data: T) -> IrqSafeSpinLock<T> {
        IrqSafeSpinLock {
            inner: SpinLock::new(data),
        }
    }

    pub fn lock(&self) -> IrqSafeGuard<T> {
        // 1. Husk om interrupts var slået til før vi rører noget
        let was_enabled = interrupts_enabled();
        // 2. Slå dem fra, så ingen interrupt kan afbryde os her.
        disable_interrupts();
        // 3. Tag den indre lås (den almindelige spin-logik)
        let guard = self.inner.lock();
        IrqSafeGuard {
            _guard: guard,
            was_enabled,
        }
    }
}

// Guarden holder den indre SpinGuard i live.
// Og hukser om interruås skal tændes igen ved drop.
pub struct IrqSafeGuard<'a,T> {
    _guard: SpinGuard<'a,T>,
    was_enabled: bool,
}

impl<'a, T> core::ops::Deref for IrqSafeGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self._guard
    }
}

impl<'a, T> core::ops::DerefMut for IrqSafeGuard<'a,T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self._guard
    }
}

impl<'a, T> Drop for IrqSafeGuard<'a, T> {
    fn drop(&mut self) {
        // Rust droppper felterne i rækkefølgen. guard frigives først
        // Gendeanner så interruåts hvis de var sleåt til før.
        // eller lader vi dem være slukket, så vi ikke tænder for dem et forkert sted. 
        if self.was_enabled {
            enable_interrupts();
        }
    }
}