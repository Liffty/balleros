use core::fmt;

// VGA text buffer konstanter
const VGA_BUFFER: usize = 0xB8000;
const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;

#[allow(dead_code)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGrey = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

// Combine foreground og background to one colorByte
pub fn color_code(foreground: Color, background: Color) -> u8 {
    (background as u8) << 4 | (foreground as u8)
}

// VGA writer - remeber cursor position and color
pub struct Writer {
    col: usize,
    row: usize,
    color: u8,
}

impl Writer {
    pub const fn new() -> Writer {
        Writer {
            col: 0,
            row: 0,
            color: 0x0F, // White on black as default
        }
    }

    pub fn set_color(&mut self, foreground: Color, background: Color) {
        self.color = color_code(foreground, background);
    }

    pub fn clear_screen(&mut self) {
        for row in 0..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                self.write_byte_at(row, col, b' ', self.color)
            }
        }
        self.col = 0;
        self.row = 0;
    }

    // write a byte on a specific position
    fn write_byte_at(&self, row: usize, col: usize, byte: u8, color: u8) {
        let offset = (row * VGA_WIDTH + col) * 2;
        let buffer = VGA_BUFFER as *mut u8;
        unsafe {
            *buffer.add(offset) = byte;
            *buffer.add(offset + 1) = color;
        }
    }

    //Write byte at specific cursor position
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.col >= VGA_WIDTH {
                    self.new_line();
                }
                self.write_byte_at(self.row, self.col, byte, self.color);
                self.col += 1;
            }
        }
    }

    // new line, move cursoer down if neccersary
    fn new_line(&mut self) {
        self.col = 0;
        self.row += 1;
        if self.row >= VGA_HEIGHT {
            self.scroll();
            self.row = VGA_HEIGHT - 1;
        }
    }

    // Scroll screen one line op
    fn scroll(&self) {
        let buffer = VGA_BUFFER as *mut u8;
        unsafe {
            for row in 1..VGA_HEIGHT {
                for col in 0..VGA_WIDTH {
                    let from = (row * VGA_WIDTH + col) * 2;
                    let to = ((row - 1) * VGA_WIDTH + col) * 2;
                    *buffer.add(to) = *buffer.add(from);
                    *buffer.add(to + 1) = *buffer.add(from + 1);
                }
            }
            // Remove the line below
            for col in 0..VGA_WIDTH {
                let offset = ((VGA_HEIGHT - 1) * VGA_WIDTH + col) * 2;
                *buffer.add(offset) = b' ';
                *buffer.add(offset + 1) = 0x0F;
            }
        }
    }
}

// implements write trait
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}
