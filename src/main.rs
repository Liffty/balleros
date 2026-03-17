#![no_std]
#![no_main]

use core::panic::PanicInfo;

// Kernl will start here - so the will be our new entry point
#[unsafe(no_mangle)] // This will prevent rust from changing the function name
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    // We write to the vga buffer as proof
    let vga_buffer = 0xB8000 as *mut u8;
    let msg = b"Hello from Rust kernel!";
    let color = 0x0F; // white on black

    for (i, &byte) in msg.iter().enumerate() {
        unsafe {
            // line 3 in VGA (2 lines used in by bootloader)
            *vga_buffer.offset((320 + i * 2) as isize) = byte;
            *vga_buffer.offset((320 + i * 2 + 1) as isize) = color;
        }
    }

    // The kernel should never return.
    loop {}
}

// Rust needs a panic handler, we don't have a std, so we need to make our own.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
