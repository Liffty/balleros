use crate::port::Port;
use core::fmt;

const COM1: u16 = 0x3F8;

pub struct Serial {
    data: Port<u8>,
    int_enable: Port<u8>,
    fifo_ctrl: Port<u8>,
    line_ctrl: Port<u8>,
    modem_ctrl: Port<u8>,
    line_status: Port<u8>,
}

impl Serial {
    pub const fn new() -> Serial {
        Serial {
            data: Port::new(COM1),
            int_enable: Port::new(COM1 + 1),
            fifo_ctrl: Port::new(COM1 + 2),
            line_ctrl: Port::new(COM1 + 3),
            modem_ctrl: Port::new(COM1 + 4),
            line_status: Port::new(COM1 + 5),
        }
    }
    
    pub fn init(&self) {
        unsafe {
            self.int_enable.write(0x00);
            self.line_ctrl.write(0x80);
            self.data.write(0x03);
            self.int_enable.write(0x00);
            self.line_ctrl.write(0x03);
            self.fifo_ctrl.write(0xC7);
            self.modem_ctrl.write(0x03);
        }
    }

    fn write_byte(&self, byte: u8) {
        unsafe {
            while self.line_status.read() & 0x20 == 0 {}
            self.data.write(byte);
            }
        }
}

impl fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
        Ok(())
    }
}


        
