use crate::port::Port;

// Master PIC. Command port 0x20, data port 0x21
// Slave PIC. Command port 0xA0, data port 0xA1
struct Pic {
    command: Port<u8>,
    data: Port<u8>,
}

// Chained master + slave pic
pub struct ChainedPics {
    master: Pic,
    slave: Pic,
}

impl ChainedPics {
    pub const fn new() -> ChainedPics {
        ChainedPics {
            master: Pic {
                command: Port::new(0x20),
                data: Port::new(0x21),
            },
            slave: Pic {
                command: Port::new(0xA0),
                data: Port::new(0xA1),
            },
        }
    }

    // Remap PIC's so IRQ 0-7 -> interrupts 32-39 and IRQ 8-15 -> interrupts 40-47
    // Mask all IRQ after (0xFF) - we enable them later individually later.
    //
    // Protocol is 4 ICW's (Initialization Command Words) to every PIC:
    // ICW1: 0x11 (00010001b) - starts the initialization sequence and tells the PICs that there will be ICW4's (0x11 = init + ICW4 needed)
    // ICW2: Interrupt vector offset. (32 for master, 40 for slaves)
    // ICW3: Tells master/slave about their relationship. (master: slave on IRQ2, slave: cascade identity 2)
    // ICW4: Exstra mode (0x01 = 8086 mode)
    pub fn remap(&self) {
        unsafe {
            // Save the current masks so we don't lose the BIOS configuration
            // (We will overwrite them later tho, just to have good docs)
            let _master_mask = self.master.data.read();
            let _slave_mask = self.slave.data.read();

            // ICW1: Start init sequence on both PICs
            self.master.command.write(0x11);
            io_wait();
            self.slave.command.write(0x11);
            io_wait();

            // ICW2: set vector offsets
            self.master.data.write(32); // IRQ 0-7 -> interrupts 32-39
            io_wait();
            self.slave.data.write(40); // IRQ 8-15 -> interrupts 40-47
            io_wait();

            // ICW3: Tell master that slave is on IRQ2 (bit 2 = 0x04)
            self.master.data.write(0x04);
            io_wait();
            // Tell slave that its cascade identity is 2 (IRQ 2 = 0x02)
            self.slave.data.write(0x02);
            io_wait();

            // ICW4: Set 8086 mode
            self.master.data.write(0x01);
            io_wait();
            self.slave.data.write(0x01);
            io_wait();

            // Mask all IRQs (0xFF = all blocked)
            self.master.data.write(0xFF);
            self.slave.data.write(0xFF);
        }
    }

    // Enable a specific IRQ by unmasking it (setting the bit to 0)
    // IRQ 0-7 are on the master, IRQ 8-15 are on the slave
    pub fn enable_irq(&self, irq: u8) {
        unsafe {
            if irq < 8 {
                let mask = self.master.data.read();
                // Clear bit on the current IRQ (0 = enabled)
                self.master.data.write(mask & !(1 << irq));
            } else {
                let mask = self.slave.data.read();
                self.slave.data.write(mask & !(1 << (irq - 8)));
                // Slave is connected to IRQ 2 on the master - should also unmask that one
                let master_mask = self.master.data.read();
                self.master.data.write(master_mask & !(1 << 2));
            }
        }
    }

    // Send End of Interrupt (EOI) to the correct PIC
    // Should be called at EVERY hardware interrupt handler
    // Or the PIC will become silent like the lambs
    pub fn send_eoi(&self, irq: u8) {
        unsafe {
            // If IRQ came from slave (8 to 15), then both PICs need EOI
            if irq >= 8 {
                self.slave.command.write(0x20);
            }
            // Master should always get EOI
            self.master.command.write(0x20);
        }
    }
}

// Short break between PIC commands
// We write to 0x80 (POST diagnostic port) that takes ~1μs
// standard I/O delay on x86.
fn io_wait() {
    unsafe {
        Port::<u8>::new(0x80).write(0);
    }
}