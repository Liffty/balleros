use core::arch::asm;

// Trait for types that can read/write via i/o ports
// every type has there own assembly instructions
// u8 -> in al, dx / out dx, al
// u16 -> in ax, dx / out dx, ax
// u32 -> in eax, dx / out dx, eax

pub trait PortAccess {
    // Read values from a given port
    unsafe fn read_port(port: u16) -> Self;
    // Write values to a given port
    unsafe fn write_port(port: u16, value: Self);
}

impl PortAccess for u8 {
    unsafe fn read_port(port: u16) -> u8 {
        let value: u8;
        unsafe {
            asm!("in al, dx", out("al") value, in("dx") port, options(nostack, nomem));
        }
        value
    }

    unsafe fn write_port(port: u16, value: u8) {
        unsafe { asm!("out dx, al", in("dx") port, in("al") value, options(nostack, nomem)) }
    }
}

impl PortAccess for u16 {
    unsafe fn read_port(port: u16) -> u16 {
        let value: u16;
        unsafe { asm!("in ax, dx", out("ax") value, in("dx") port, options(nostack, nomem)) }
        value
    }

    unsafe fn write_port(port: u16, value: u16) {
        unsafe { asm!("out dx, ax", in("dx") port, in("ax") value, options(nostack, nomem)) }
    }
}

// A I/O-port bunded to a specific portnumber
// Type T dicates the size of the read/write operation

pub struct Port<T: PortAccess> {
    port: u16,
    _phantom: core::marker::PhantomData<T>,
}

impl<T: PortAccess> Port<T> {
    // Create a new port with a given number
    // the creation it self is safe but the read and write is unsafe
    pub const fn new(port: u16) -> Port<T> {
        Port {
            port,
            _phantom: core::marker::PhantomData,
        }
    }

    // Read value from port
    pub unsafe fn read(&self) -> T {
        unsafe { T::read_port(self.port) }
    }

    // Write to port
    pub unsafe fn write(&self, value: T) {
        unsafe { T::write_port(self.port, value) }
    }
}
