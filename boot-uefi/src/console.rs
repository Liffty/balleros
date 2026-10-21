use crate::{Char16, SimpleTextOutputProtocol};
use core::fmt::{self, Write};

#[derive(Clone, Copy)]
pub struct ConOut {
    out: *const SimpleTextOutputProtocol,
}

impl ConOut {
    pub fn new(out: *const SimpleTextOutputProtocol) -> ConOut {
        ConOut { out }
    }

    pub fn fail(mut self, step: u64) -> ! {
        let _ = write!(self, "ERR step {}\n", step);
        loop {}
    }

    fn flush(&self, buf: &mut [Char16; 65], len: usize) {
        buf[len] = 0;
        unsafe {
            ((*self.out).output_string)(self.out, buf.as_ptr());
        }
    }
}

impl fmt::Write for ConOut {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut buf = [0 as Char16; 65];
        let mut len = 0;
        for unit in s.encode_utf16() {
            if unit == b'\n' as u16 {
                buf[len] = b'\r' as u16;
                len += 1;
            }
            buf[len] = unit;
            len += 1;
            if len >= 63 {
                self.flush(&mut buf, len);
                len = 0;
            }
        }
        if len > 0 {
            self.flush(&mut buf, len);
        }
        Ok(())
    }
}
