#![no_std]
#![no_main]

mod idt;
mod pic;
mod port;
mod vga;
mod sync;

use core::fmt::Write;
use core::panic::PanicInfo;
use pic::ChainedPics;
use port::Port;
use vga::{Color, Writer};
use sync::IrqSafeSpinLock;



#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {


    let mut writer = Writer::new();
    writer.clear_screen();
    writer.set_color(Color::LightGreen, Color::Black);

    static TEST_LOCK: IrqSafeSpinLock<u32> = IrqSafeSpinLock::new(0);
    {
    let mut val = TEST_LOCK.lock();
    *val += 1;
    write!(writer, "Lock test: {}\n", *val).unwrap();
    }


    write!(writer, "Welcome to BallerOS!\n").unwrap();

    let mut interrupt_table = idt::Idt::new();
    idt::register_exception_handlers(&mut interrupt_table);
    interrupt_table.load();

    write!(writer, "IDT loaded: 256 entries\n").unwrap();

    writer.set_color(Color::White, Color::Black);
    write!(writer, "VGA driver loaded.\n").unwrap();
    write!(writer, "Screen: {}x{} characters\n", 80, 25).unwrap();

    let pics = ChainedPics::new();
    pics.remap();
    write!(writer, "PIC remapped: IRQ 0-15 -> INT 32-47\n").unwrap();

    loop {}
}

// Rust needs a panic handler, we don't have a std, so we need to make our own.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
