#![no_std]
#![no_main]

use core::panic::PanicInfo;

// === Minimal UEFI-typer, håndskrevet ===
// Vi definerer kun lige det vi skal bruge for at nå ConOut->OutputString.
// Alt er pointere ind i strukturer firmwaren giver os på runtime.

type Char16 = u16;
type Status = usize; // EFI_STATUS er en usize; 0 = EFI_SUCCESS

#[repr(C)]
pub struct SimpleTextOutputProtocol {
    reset: usize, // funktionspointer - vi bruger den ikke, så bare usize-plads
    // output_string(this, *const Char16) -> Status
    output_string: extern "efiapi" fn(*const SimpleTextOutputProtocol, *const Char16) -> Status,
    // resten af protokollen findes, men vi behøver den ikke - udelader for nu
}

// System Table: firmwaren giver os en pointer til denne.
// Felterne før con_out er header + andre pointere vi ikke bruger endnu.
// Vi lægger padding så con_out lander på det rigtige offset.
#[repr(C)]
pub struct SystemTable {
    _header: [u8; 24],      // EFI_TABLE_HEADER (24 bytes)
    _fw_vendor: usize,      // *const Char16
    _fw_revision: u32,
    _padding: u32,          // alignment til 8-byte grænse
    _con_in_handle: usize,
    _con_in: usize,
    _con_out_handle: usize,
    con_out: *const SimpleTextOutputProtocol, // <- den vi vil have fat i
    // ... flere felter efter denne, men vi stopper her
}

#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(_image_handle: usize, system_table: *const SystemTable) -> Status {
    // UTF-16-streng, null-termineret. UEFI-tekst er Char16, ikke u8.
    let msg: &[Char16] = &[
        b'H' as u16, b'e' as u16, b'l' as u16, b'l' as u16, b'o' as u16,
        b' ' as u16,
        b'f' as u16, b'r' as u16, b'o' as u16, b'm' as u16,
        b' ' as u16,
        b'U' as u16, b'E' as u16, b'F' as u16, b'I' as u16,
        b'\r' as u16, b'\n' as u16,
        0, // null-terminator
    ];

    unsafe {
        let st = &*system_table;
        let con_out = &*st.con_out;
        (con_out.output_string)(st.con_out, msg.as_ptr());
    }

    // Boot services stopper ikke af sig selv; vi hænger bare her.
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}