use core::str;

use crate::port::Port;

// Master PIC. Command port 0x20, data port 0x21
// Slave PIC. Command port 0xA0, data port 0xA21
struct Pic {
    command: Port<u8>,
    data: Port<u8>,
}

// Chainded master + slave pic
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
