#![no_std]
#![no_main]

mod idt;
mod port;
mod vga;

use core::fmt::Write;
use core::panic::PanicInfo;
use port::Port;
use vga::{Color, Writer};

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    let mut writer = Writer::new();
    writer.clear_screen();
    writer.set_color(Color::LightGreen, Color::Black);

    write!(writer, "Welcome to BallerOS!\n").unwrap();

    let mut interrupt_table = idt::Idt::new();
    idt::register_exception_handlers(&mut interrupt_table);
    interrupt_table.load();

    write!(writer, "IDT loaded: 256 entries\n").unwrap();

    writer.set_color(Color::White, Color::Black);
    write!(writer, "VGA driver loaded.\n").unwrap();
    write!(writer, "Screen: {}x{} characters\n", 80, 25).unwrap();

    let ps2_status = Port::<u8>::new(0x64);
    let status = unsafe { ps2_status.read() };
    write!(writer, "PS/2 status port: 0x{:02X}\n", status).unwrap();

    loop {}
}

// Rust needs a panic handler, we don't have a std, so we need to make our own.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
