// Boot lige nu er:
// qemu-system-x86_64 \
//  -drive if=pflash,format=raw,readonly=on,file=/opt/homebrew/Cellar/qemu/10.2.1/share/qemu/edk2-x86_64-code.fd \
//  -drive format=raw,file=fat:rw:boot-uefi/esp \
//  -net none


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


// === EFI_HEADER_TABLE (24 bytes) ===
#[repr(C)]
pub struct TableHeader {
    signature: u64,
    revision: u32,
    header_size: u32,
    crc32: u32,
    reserved: u32,
}

// === Boot services ===
// Function-pointere i SPEC rækkefølgen.
#[repr(C)]
pub struct BootServices {
    _header: TableHeader,

    // --- Task Priority services ---
    _raise_tpl: usize,
    _restore_tpl: usize,
    
    //  --- Memory Services ---
    allocate_pages: extern "efiapi" fn(
        alloc_type: u32, // EFI_ALLOCATE_TYPE (0=AnyPages, 1=MaxAddress, 2=Address)
        mem_type: u32,  // EFI_MEMORY_TYPE (Vi bruger 2 = LoaderData)
        pages: usize, // antal 4KB-sider
        memory: *mut u64, // out: fysisk adresse på allokeret område
    ) -> Status,
    _free_pages: usize,
    _get_memory_map: usize,
    allocate_pool: extern "efiapi" fn(
        pool_type: u32, // EFI_MEMORY_TYPE (2=LoaderData)
        size: usize,
        buffer: *mut *mut u8, // peger på allokeret område
    ) -> Status,
    _free_pool: extern "efiapi" fn(buffer: *mut u8) -> Status,

    // --- Event & Timer Services ---
    _create_event: usize,
    _set_timer: usize,
    _wait_for_event: usize,
    _signal_event: usize,
    _close_signal: usize,
    _check_event: usize,

    // ---Protocol Handler Services ---
    _install_protocol_interface: usize,
    _reinstall_protocol_interface: usize,
    _uninstall_protocol_interface: usize,
    handle_protocol:
        extern "efiapi" fn(handle: usize, protocol: *const Guid, interface: *mut *mut u8) -> Status,
    _reserved: usize,
    _register_protocol_notify: usize,
    _locate_handle: usize,
    _locate_device_path: usize,
    _install_configuration_table: usize,

    // --- Image Services ---
    _load_image: usize,
    _start_image: usize,
    _exit: usize,
    _unload_image: usize,
    _exit_boot_services: usize,

    // TDOD skriv resten af tabellen.
}

#[repr(C)]
pub struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}
    
const LOADED_IMAGE_GUID: Guid = Guid {
    data1: 0x5B1B31A1, data2: 0x9562, data3: 0x11D2,
    data4: [0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B]
};

const SIMPLE_FILE_SYSTEM_GUID: Guid = Guid {
    data1: 0x964E5B22, data2: 0x6459, data3: 0x11D2,
    data4: [0x8E, 0x39, 0x00,  0xA0, 0xC9, 0x69, 0x72, 0x3B],
};

const FILE_INFO_GUID: Guid = Guid {
    data1: 0x09576E92, data2: 0x6D3F, data3: 0x11D2,
    data4: [0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B]
};

// Loadede Image - vi vil kun have device_handle ud (hvilken disk vi kom fra)
#[repr(C)]
pub struct LoadedImageProtocol {
    _revision: u32,
    _padding: u32,
    _parent_handle: usize,
    _system_table: usize,
    device_handle: usize,
}


#[repr(C)]
pub struct SimpleFileSystemProtocol {
    _revision: u64,
    open_volume: extern "efiapi" fn(
        this: *const SimpleFileSystemProtocol,
        root: *mut *mut FileProtocol,
    ) -> Status,
}


#[repr(C)]
pub struct FileProtocol {
    _revision: u64,
    open: extern "efiapi" fn(
        this: *const FileProtocol,
        new_handle: *mut *mut FileProtocol,
        filename: *const Char16,
        open_mode: u64,
        attributes: u64,
    ) -> Status,
    close: extern "efiapi" fn(this: *const FileProtocol) -> Status,
    _delete: usize,
    read: extern "efiapi" fn(
        this: *const FileProtocol,
        buffer_size: *mut usize,
        buffer: *mut u8,
    ) -> Status,
    _write: usize,
    _get_position: usize,
    _set_position: usize,
    get_info: extern "efiapi" fn(
        this: *const FileProtocol,
        info_type: *const Guid,
        buffer_size: *mut usize,
        buffer: *mut u8,
    ) -> Status,
}

#[repr(C)]
pub struct SystemTable {
    _header: TableHeader,       // 24 bytes, var før [u8; 24]
    _fw_vendor: usize,
    _fw_revision: u32,
    _padding: u32,
    _con_in_handle: usize,
    _con_in: usize,
    _con_out_handle: usize,
    con_out: *const SimpleTextOutputProtocol,
    _std_err_handle: usize,
    _std_err: usize,
    _runtime_services: usize,
    boot_services: *const BootServices
}

fn u64_to_utf16(mut n: u64, buf: &mut [Char16; 21]) -> &[Char16] {
    if n == 0 {
        buf[0] = b'0' as u16;
        buf[1] = 0;
        return &buf[..2];
    }

    let mut digits = [0u8; 20];
    let mut count = 0;
    while n > 0 {
        digits[count] = (n % 10) as u8;
        n /= 10;
        count += 1;
    }

    for i in 0..count {
        buf[i] = (b'0' + digits[count -1 - i]) as u16;
    }
    buf[count] = 0;
    &buf[..count + 1]
}

fn u8_to_hex(byte: u8, buf: &mut [Char16; 3]) -> &[Char16] {
    let hex = b"0123456789ABCDEF";
    buf[0] = hex[(byte >> 4) as usize] as u16;
    buf[1] = hex[(byte & 0x0F) as usize] as u16;
    buf[2] = 0;
    &buf[..3]
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

    let prefix_err: &[Char16] = &[
        b'E' as u16, b'R' as u16, b'R' as u16, b' ' as u16,
        b's' as u16, b't' as u16, b'e' as u16, b'p' as u16, b' ' as u16, 0,
    ];

    unsafe {
        let st = &*system_table;
        let bs = &*st.boot_services;
        let con_out = &*st.con_out;

        let print =|s: *const Char16| {
            (con_out.output_string)(st.con_out, s);
        };

        // ---1. Loaded Image Protocol: hvilken disk kom vi fra? ---
        let mut loaded_image: *mut u8 = core::ptr::null_mut();
        let status = (bs.handle_protocol)(
            _image_handle,
            &LOADED_IMAGE_GUID,
            &mut loaded_image,
        );
        if status != 0 {
            let mut b = [0u16; 21];
            print(prefix_err.as_ptr());
            print(u64_to_utf16(1, &mut b).as_ptr());
            loop{}
        }
        let li = &*(loaded_image as *const LoadedImageProtocol);
        let device_handle = li.device_handle;

        // --- 2. Simple File System Protocol på den disk ---
        let mut sfs: *mut u8 = core::ptr::null_mut();
        let status = (bs.handle_protocol)(
            device_handle,
            &SIMPLE_FILE_SYSTEM_GUID,
            &mut sfs,
        );
        if status != 0 {
            let mut b = [0u16; 21];
            print(prefix_err.as_ptr());
            print(u64_to_utf16(2, &mut b).as_ptr());
            loop{}
        }
        let mut root: *mut FileProtocol = core::ptr::null_mut();
        let sfs = &*(sfs as *const SimpleFileSystemProtocol); 
        let status = (sfs.open_volume)(sfs, &mut root);
        if status != 0 {
            let mut b = [0u16; 21];
            print(prefix_err.as_ptr());
            print(u64_to_utf16(3, &mut b).as_ptr());
            loop{}
        }
        let root = &*root;

        // --- 4. Åbn KERNEL.BIN ---
        // UTF-16 filnavn, null-termineret
        let name: &[Char16] = &[
            b'K' as u16, b'E' as u16, b'R' as u16, b'N' as u16,
            b'E' as u16, b'L' as u16, b'.' as u16,
            b'B' as u16, b'I' as u16, b'N' as u16, 0, 
        ];
        let mut file: *mut FileProtocol = core::ptr::null_mut();
        let status = (root.open)(root, &mut file, name.as_ptr(), 1, 0);
        if status != 0 {
            let mut b = [0u16; 21];
            print(prefix_err.as_ptr());
            print(u64_to_utf16(4, &mut b).as_ptr());
            loop{}
        }
        let file = &*file;

        // --- 5. getinfo: dansen for filstørrelsen ---
        // første kald: buffer_size = 0 -> forventer buffer_too_small,
        // og buffer_size opdateres til den krævede størrelse.
        let mut info_size: usize = 0;
        let _ = (file.get_info)(
            file,
            &FILE_INFO_GUID,
            &mut info_size,
            core::ptr::null_mut(),
        );
        // alloker den krævede plads via allocate_pool
        let mut info_buf: *mut u8 = core::ptr::null_mut();
        let status = (bs.allocate_pool)(2, info_size, &mut info_buf); // 2 = loaderdata
        if status != 0 {
            let mut b = [0u16; 21];
            print(prefix_err.as_ptr());
            print(u64_to_utf16(5, &mut b).as_ptr());
            loop{}
        }                                           
        // andet kald
        let status = (file.get_info)(
            file,
            &FILE_INFO_GUID,
            &mut info_size,
            info_buf
        );
        if status != 0 {
            let mut b = [0u16; 21];
            print(prefix_err.as_ptr());
            print(u64_to_utf16(6, &mut b).as_ptr());
            loop{}
        }

        // filesize ligger på offset 8 i efi_file_info
        let file_size = *(info_buf.add(8) as *const u64);

        // --- 6 print filstørrelsen ---
// Print: "KERNEL.BIN: <størrelse> bytes"
        let label: &[Char16] = &[
            b'K' as u16,b'E' as u16,b'R' as u16,b'N' as u16,b'E' as u16,b'L' as u16,
            b'.' as u16,b'B' as u16,b'I' as u16,b'N' as u16,b':' as u16,b' ' as u16, 0,
        ];
        let suffix: &[Char16] = &[
            b' ' as u16,b'b' as u16,b'y' as u16,b't' as u16,b'e' as u16,b's' as u16,
            b'\r' as u16,b'\n' as u16, 0,
        ];
        let mut num_buf = [0u16; 21];
        print(label.as_ptr());
        print(u64_to_utf16(file_size, &mut num_buf).as_ptr());
        print(suffix.as_ptr());

        // --- 7. Alloket buffer til fil-indhold ---
        // AnyPages: firmware vælger adressen (scratch-buffe, som besluttet for lag 1)
        let pages = (file_size as usize + 4095) / 4096;
        let mut kernel_addr: u64 = 0;
        let status = (bs.allocate_pages)(0,2,pages,&mut kernel_addr);
        if status != 0 {
            let mut b = [0u16; 21];
            print(prefix_err.as_ptr());
            print(u64_to_utf16(7, &mut b).as_ptr());
            loop {}
        }

        // --- 8 læs filen ind i bufferen
        // buffer_size er in/out: vi giver filstørrelsen ind, får læste bytes ud
        let mut read_size = file_size as usize;
        let status = (file.read)(file, &mut read_size, kernel_addr as *mut u8);
        if status != 0 {
            let mut b = [0u16; 21];
            print(prefix_err.as_ptr());
            print(u64_to_utf16(8, &mut b).as_ptr());
            loop {}
        }

        let ok: &[Char16] = &[
            b'R' as u16,b'e' as u16, b'a' as u16, b'd' as u16, b' ' as u16,
            b'O' as u16,b'K' as u16, b':' as u16,b' ' as u16, 0,
        ];
        let space: &[Char16] = &[b' ' as u16, 0];
        print(ok.as_ptr());

        let data = kernel_addr as *const u8;
        for i in 0..4 {
            let byte = *data.add(i);
            let mut hb = [0u16; 3];
            print(u8_to_hex(byte, &mut hb).as_ptr());
            print(space.as_ptr());
        }

    // boot services stopper ikke af sig selv; vi hænger bare her.
    loop {}
    }
} 

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
