use core::arch::{asm, naked_asm};

// One IDT entry (16 bytes, as the CPU expects)
#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct IdtEntry {
    offset_low: u16,  // handler adresse bit 0-15
    selector: u16,    // code segment selector (0x08 fra vores GDT)
    ist: u8,          // interrupt stack table offset (0 for nu)
    type_attr: u8,    // type and attributes
    offset_mid: u16,  // handler adress bit 16-31
    offset_high: u32, // handler adress bit 32-63
    reserved: u32,    // always 0
}

impl IdtEntry {
    //empty entry
    pub const fn empty() -> IdtEntry {
        IdtEntry {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    // place a handler for this entry
    pub fn set_handler(&mut self, handler: u64) {
        self.offset_low = handler as u16;
        self.offset_mid = (handler >> 16) as u16;
        self.offset_high = (handler >> 32) as u32;
        self.selector = 0x08; // Code segment in our GDT
        self.ist = 0;
        // 0x8E = present (bit 7) + DPL 0 (bit 5-6) + intterupt gate (0xE)
        self.type_attr = 0x8E;
        self.reserved = 0;
    }
}

// IDT Table - 256 entries
pub struct Idt {
    entries: [IdtEntry; 256],
}

// IDT Descriptor (gives to lidt instructions)
#[repr(C, packed)]
struct IdtDescriptor {
    size: u16,   // size in bytes minus 1
    offset: u64, // adresse on the IDT
}

impl Idt {
    pub const fn new() -> Idt {
        Idt {
            entries: [IdtEntry::empty(); 256],
        }
    }

    // set handler for specific interupt number
    pub fn set_handler(&mut self, index: u8, handler: extern "C" fn()) {
        self.entries[index as usize].set_handler(handler as *const () as u64);
    }

    // Laod IDT'en in CPU via lidt
    pub fn load(&self) {
        let descriptor = IdtDescriptor {
            size: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
            offset: self.entries.as_ptr() as u64,
        };

        unsafe {
            asm!("lidt [{}]", in(reg) &descriptor, options(nostack));
        }
    }
}

// ===========
// interupt handler stubs
// ===========
// Every handler will: save register -> call Rust code -> reentry -> iretq
// We need a naked function from rust so we an write untoched assembly

// Generic exception handler - prints interrupt-number and stops
macro_rules! exception_handler {
($name:ident, $number:expr, $msg:expr) => {
    #[unsafe(naked)]
    pub extern "C" fn $name() {
        naked_asm!(
            // Save all the general purpose registers
            "push rax",
            "push rbx",
            "push rcx",
            "push rdx",
            "push rsi",
            "push rdi",
            "push rbp",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            // Call Rust handler with the interupt number as argument
            "mov rdi, {number}",
            "call {rust_handler}",
            // reentry register
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rbp",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rbx",
            "pop rax",
            "iretq",
            number = const $number,
            rust_handler = sym generic_exceptoin_handler,
        );
    }
};
}

// Rust - function that vil be called by exception handlers
// Printer error message to the VGA and stops
extern "C" fn generic_exceptoin_handler(interrupt_number: u64) {
    let vga = 0xB8000 as *mut u8;
    let msg = b"EXCEPTION #";
    let color = 0x4F; // white on read

    // write "EXCEPTION #" on line 24 (lowest line)
    let row_offset = 24 * 80 * 2;
    for (i, &byte) in msg.iter().enumerate() {
        unsafe {
            *vga.add(row_offset + i * 2) = byte;
            *vga.add(row_offset + i * 2 + 1) = color;
        }
    }

    // Write interrupt numnber (simmple 2 digit conversion)
    let tens = (interrupt_number / 10) as u8 + b'0';
    let ones = (interrupt_number % 10) as u8 + b'0';
    let pos = row_offset + msg.len() * 2;
    unsafe {
        *vga.add(pos) = tens;
        *vga.add(pos + 1) = color;
        *vga.add(pos + 2) = ones;
        *vga.add(pos + 3) = color;
    }
    // Halt - we can't continue after most exception, so we just halt
    loop {}
}

//Generate handler for CPU exceptions (0-31)
exception_handler!(exception_0, 0, "Division Error");
exception_handler!(exception_1, 1, "Debug");
exception_handler!(exception_2, 2, "NMI");
exception_handler!(exception_3, 3, "Breakpoint");
exception_handler!(exception_4, 4, "Overflow");
exception_handler!(exception_5, 5, "Bound range");
exception_handler!(exception_6, 6, "Invalid Opcode");
exception_handler!(exception_7, 7, "Device Not Available");
exception_handler!(exception_8, 8, "Double Fault");
exception_handler!(exception_9, 9, "Coprocessor Segment");
exception_handler!(exception_10, 10, "Invalid TSS");
exception_handler!(exception_11, 11, "Segment not present");
exception_handler!(exception_12, 12, "Stack-segment fault");
exception_handler!(exception_13, 13, "General Protection");
exception_handler!(exception_14, 14, "Page Fault");
exception_handler!(exception_15, 15, "Reserved");
exception_handler!(exception_16, 16, "x87 Float Error");
exception_handler!(exception_17, 17, "Alignment Check");
exception_handler!(exception_18, 18, "Machine Check");
exception_handler!(exception_19, 19, "SIMD Float Error");
exception_handler!(exception_20, 20, "Virtualization");

// register all the exception handlers in the IDT
pub fn register_exception_handlers(idt: &mut Idt) {
    let handlers: [extern "C" fn(); 21] = [
        exception_0,
        exception_1,
        exception_2,
        exception_3,
        exception_4,
        exception_5,
        exception_6,
        exception_7,
        exception_8,
        exception_9,
        exception_10,
        exception_11,
        exception_12,
        exception_13,
        exception_14,
        exception_15,
        exception_16,
        exception_17,
        exception_18,
        exception_19,
        exception_20,
    ];

    for (i, &handler) in handlers.iter().enumerate() {
        idt.set_handler(i as u8, handler);
    }
}
