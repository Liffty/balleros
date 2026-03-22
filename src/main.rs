#![no_std]
#![no_main]

mod vga;

use core::fmt::Write;
use core::panic::PanicInfo;
use vga::{Color, Writer};

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    let mut writer = Writer::new();
    writer.clear_screen();
    writer.set_color(Color::LightGreen, Color::Black);

    write!(writer, "Welcome to BallerOS!\n").unwrap();

    writer.set_color(Color::White, Color::Black);
    write!(writer, "VGA driver loaded.\n").unwrap();
    write!(writer, "Screen: {}x{} characters\n", 80, 25).unwrap();

    loop {}
}

// Rust needs a panic handler, we don't have a std, so we need to make our own.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
