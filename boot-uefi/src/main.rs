// Boot lige nu er:
// qemu-system-x86_64 \
//  -drive if=pflash,format=raw,readonly=on,file=/opt/homebrew/Cellar/qemu/10.2.1/share/qemu/edk2-x86_64-code.fd \
//  -drive format=raw,file=fat:rw:boot-uefi/esp \
//  -net none


#![no_std]
#![no_main]

mod console;
mod paging;

use console::ConOut;
use core::fmt::Write;
use core::panic::PanicInfo;
use paging::{NO_EXECUTE, SIZE_2M, SIZE_4K, WRITABLE};

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

// === Memory map ===
// En indgang i UEFI memory map
// firmare kan bruge store descriptor_size end sizeof
// Spring altid frem med descriptor_size ikke med size of
#[repr(C)]
pub struct MemoryDescriptor {
    mem_type: u32,
    _padding: u32,
    phys_start: u64,
    _virt_start: u64,
    num_pages: u64,
    _attribute: u64,
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
    get_memory_map: extern "efiapi" fn(
        map_size: *mut usize,
        map: *mut u8,
        map_key: *mut usize,
        descripter_size: *mut usize,
        descriptor_version: *mut u32,
    ) -> Status,
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
    _get_next_monotonic_count: usize,
    _stall: usize,
    _set_watchdog_timer: usize,

    // --- DriverSupport Services ---
    _connect_controller: usize,
    _disconnect_controller: usize,

    // --- Open/Close Protocol Services ---
    _open_protocol: usize,
    _close_protocol: usize,
    _open_protocol_information: usize,

    // --- Library Services ---
    _protocols_per_handle: usize,
    _locate_handle_buffer: usize,
    locate_protocol: extern "efiapi" fn(
        protocol: *const Guid,
        registration: *mut u8,  // NULL for os
        interface: *mut *mut u8,    // out: protocol-pointer
    ) -> Status,
    // resten af tabellen findes, men vi stopper her
}

#[repr(C)]
pub struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

const GRAPHICS_OUTPUT_GUID: Guid = Guid {
    data1: 0x9042A9DE, data2: 0x23DC, data3: 0x4A38,
    data4: [0x96, 0xFB, 0x7A, 0xDE, 0xD0, 0x80, 0x51, 0x6A],
};
    
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

// Pixel-info for en video-mode
#[repr(C)]
pub struct GopModeInfo {
    _version: u32,
    horizontal_resolution: u32,
    vertical_resolution: u32,
    pixel_format: u32,
    _pixel_bitmask: [u32; 4],
    pixels_per_scan_line: u32,
}

// GOP's Mode-strut: peger til info + selve frambufferen.
#[repr(C)]
pub struct GopMode {
    _max_mode: u32,
    _mode: u32,
    info: *const GopModeInfo,
    _size_of_info: usize,
    frame_buffer_base: u64, // fysisk adresse på frambufferen
    frame_buffer_size: usize,
}

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

// Graphics Output Protocol
#[repr(C)]
pub struct GraphicsOutputProtocol {
    _query_mode: usize,
    _set_mode: usize,
    _blt: usize,
    mode: *const GopMode, // pointer. Not function
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

// === ELF64 ===
#[repr(C)]
pub struct Elf64Header {
    ident: [u8; 16],
    _elf_type: u16,
    machine: u16,
    _version: u32,
    entry: u64,
    phoff: u64,
    _shoff: u64,
    _flags: u32,
    _ehsize: u16,
    phentsize: u16,
    phnum: u16,
    _shentsize: u16,
    _shnum: u16,
    _shstrndx: u16,
}

#[repr(C)]
pub struct Elf64ProgramHeader {
    p_type: u32,
    flags: u32,
    offset: u64,
    vaddr: u64,
    _paddr: u64,
    filesz: u64,
    memsz: u64,
    _align: u64,
}

const PT_LOAD: u32 = 1;
const KERNEL_VMA: u64 = 0xFFFF_FFFF_8000_0000;



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

#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(_image_handle: usize, system_table: *const SystemTable) -> Status {
    unsafe {
        let st = &*system_table;
        let bs = &*st.boot_services;
        let mut con = ConOut::new(st.con_out);

        // ---1. Loaded Image Protocol: hvilken disk kom vi fra? ---
        let mut loaded_image: *mut u8 = core::ptr::null_mut();
        let status = (bs.handle_protocol)(_image_handle, &LOADED_IMAGE_GUID, &mut loaded_image);
        if status != 0 {
            con.fail_status(1, "LoadedImage Protocol", status);
        }
        let li = &*(loaded_image as *const LoadedImageProtocol);
        let device_handle = li.device_handle;

        // --- 2. Simple File System Protocol på den disk ---
        let mut sfs: *mut u8 = core::ptr::null_mut();
        let status = (bs.handle_protocol)(device_handle, &SIMPLE_FILE_SYSTEM_GUID, &mut sfs);
        if status != 0 {
            con.fail_status(2, "SimpleFileSystem Protocol", status);
        }
        let mut root: *mut FileProtocol = core::ptr::null_mut();
        let sfs = &*(sfs as *const SimpleFileSystemProtocol);
        let status = (sfs.open_volume)(sfs, &mut root);
        if status != 0 {
            con.fail_status(3, "open_volume", status);
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
            con.fail_status(4, "open KERNEL_BIN", status);
        }
        let file = &*file;

        // --- 5. getinfo: dansen for filstørrelsen ---
        // første kald: buffer_size = 0 -> forventer buffer_too_small,
        // og buffer_size opdateres til den krævede størrelse.
        let mut info_size: usize = 0;
        let _ = (file.get_info)(file, &FILE_INFO_GUID, &mut info_size, core::ptr::null_mut());
        // alloker den krævede plads via allocate_pool
        let mut info_buf: *mut u8 = core::ptr::null_mut();
        let status = (bs.allocate_pool)(2, info_size, &mut info_buf); // 2 = loaderdata
        if status != 0 {
            con.fail_status(5, "alloc file info", status);
        }
        // andet kald
        let status = (file.get_info)(file, &FILE_INFO_GUID, &mut info_size, info_buf);
        if status != 0 {
            con.fail_status(6, "get_info KERNEL_BIN", status);
        }

        // filesize ligger på offset 8 i efi_file_info
        let file_size = *(info_buf.add(8) as *const u64);

        // --- 6 print filstørrelsen ---
        let _ = write!(con, "KERNEL.BIN: {} bytes\n", file_size);

        // --- 7. Alloker buffer til fil-indhold ---
        // AnyPages: firmware vælger adressen (scratch-buffer, som besluttet for lag 1)
        let pages = (file_size as usize + 4095) / 4096;
        let mut kernel_addr: u64 = 0;
        let status = (bs.allocate_pages)(0, 2, pages, &mut kernel_addr);
        if status != 0 {
            con.fail_status(7, "alloc kernel file buffer", status);
        }

        // --- 8 læs filen ind i bufferen
        // buffer_size er in/out: vi giver filstørrelsen ind, får læste bytes ud
        let mut read_size = file_size as usize;
        let status = (file.read)(file, &mut read_size, kernel_addr as *mut u8);
        if status != 0 {
            con.fail_status(8, "read KERNEL_BIN", status);
        }
        let data = kernel_addr as *const u8;
        let _ = write!(
            con,
            "Read OK: {:02X} {:02X} {:02X} {:02X}\n",
            *data, *data.add(1), *data.add(2), *data.add(3)
        );

        // --- 9 Graphics Output Protocol
        let mut gop: *mut u8 = core::ptr::null_mut();
        let status = (bs.locate_protocol)(&GRAPHICS_OUTPUT_GUID, core::ptr::null_mut(), &mut gop);
        if status != 0 {
            con.fail_status(9, "locate GOP", status);
        }
        let gop = &*(gop as *const GraphicsOutputProtocol);
        let mode = &*gop.mode;
        let info = &*mode.info;

        if info.pixel_format != 1 {
            con.fail(10, "GOP pixel format not BGR");
        }

        let width = info.horizontal_resolution;
        let height = info.vertical_resolution;
        let stride = info.pixels_per_scan_line;
        let fb = mode.frame_buffer_base as *mut u32;

        // --- 10. print opløsning
        let _ = write!(con, "GOP: {}x{}\n", width, height);

        // --- 11. TestMaling
        let x0 = width / 2 - 100;
        let y0 = height / 2 - 50;
        for y in y0..y0 + 100 {
            for x in x0..x0 + 200 {
                fb.add((y * stride + x) as usize).write_volatile(0x00FF0000);
            }
        }

        // --- 12. Valider ELF-headeren
        let elf = &*(kernel_addr as *const Elf64Header);
        let valid = elf.ident[0..4] == [0x7F, b'E', b'L', b'F']
            && elf.ident[4] == 2
            && elf.ident[5] == 1
            && elf.machine == 0x3E
            && elf.phentsize as usize == core::mem::size_of::<Elf64ProgramHeader>()
            && elf.phoff + elf.phnum as u64 * elf.phentsize as u64 <= file_size;
        if !valid {
            con.fail(11, "invalid ELF header");
        }
        let phdrs = core::slice::from_raw_parts(
            (kernel_addr + elf.phoff) as *const Elf64ProgramHeader,
            elf.phnum as usize,
        );

        // --- 13
        let mut v_start = u64::MAX;
        let mut v_end = 0u64;
        for ph in phdrs {
            if ph.p_type != PT_LOAD {
                continue;
            }
            v_start = v_start.min(ph.vaddr);
            v_end = v_end.max(ph.vaddr + ph.memsz);
        }
        if v_start != KERNEL_VMA {
            con.fail(12, "kernel not linked at KERNEL_VMA");
        }
        let kernel_pages = ((v_end - v_start + 4095) / 4096) as usize;

        // --- 14
        let mut kernel_phys: u64 = 0;
        let status = (bs.allocate_pages)(0, 2, kernel_pages, &mut kernel_phys);
        if status != 0 {
            con.fail_status(13, "alloc kernel memory", status);
        }
        core::ptr::write_bytes(kernel_phys as *mut u8, 0, kernel_pages * 4096);

        // --- 15
        for ph in phdrs {
            if ph.p_type != PT_LOAD {
                continue;
            }
            if ph.offset + ph.filesz > file_size {
                con.fail(14, "ELF segment outside file");
            }
            let src = (kernel_addr + ph.offset) as *const u8;
            let dst = (kernel_phys + (ph.vaddr - KERNEL_VMA)) as *mut u8;
            core::ptr::copy_nonoverlapping(src, dst, ph.filesz as usize);
        }

        // --- 16
        let _ = write!(
            con,
            "ELF: {} pages @ {:#018x} entry {:#x}\n",
            kernel_pages, kernel_phys, elf.entry
        );

        // --- 17 mem map, første kald giver krævet størrelse
        let mut mmap_size: usize = 0;
        let mut map_key: usize = 0;
        let mut desc_size: usize = 0;
        let mut desc_version: u32 = 0;
        let _ = (bs.get_memory_map)(
            &mut mmap_size,
            core::ptr::null_mut(),
            &mut map_key,
            &mut desc_size,
            &mut desc_version,
        );

        let mmap_capacity = mmap_size + 32 * desc_size;
        let mut mmap_buf: *mut u8 = core::ptr::null_mut();
        let status = (bs.allocate_pool)(2, mmap_capacity, &mut mmap_buf);
        if status != 0 {
            con.fail_status(15, "alloc memory map buffer", status);
        }

        // andet kald: nu med buffer, så får vi selve mappen
        mmap_size = mmap_capacity;
        let status = (bs.get_memory_map)(
            &mut mmap_size,
            mmap_buf,
            &mut map_key,
            &mut desc_size,
            &mut desc_version,
        );
        if status != 0 {
            con.fail_status(16, "get_memory_map", status);
        }

        // --- 18. Gå mappen igennem: antal indgange og højeste fysiske adresse ---
        let entries = mmap_size / desc_size;
        let mut max_phys: u64 = 0;
        for i in 0..entries {
            let d = &*(mmap_buf.add(i * desc_size) as *const MemoryDescriptor);
            max_phys = max_phys.max(d.phys_start + d.num_pages * 4096);
        }
        let _ = write!(con, "MMAP: {} entries, top {:#018x}\n", entries, max_phys);

        // --- 19. Byg page tables ---
        // Hver tabel er én nulstillet side, markeret som LoaderData,
        // så kernen senere kan se i memory map at de er i brug.
        // `move`: closuren får sin egen kopi af con, så vi stadig kan bruge write! herude.
        let alloc_table = move || -> u64 {
            let mut page: u64 = 0;
            let status = (bs.allocate_pages)(0, 2, 1, &mut page);
            if status != 0 {
                con.fail_status(17, "alloc page table", status);
            }
            core::ptr::write_bytes(page as *mut u8, 0, 4096);
            page
        };
        let mut pt = paging::PageTables::new(alloc_table);

        // Identity map: alle indgange undtagen Reserved (0) og Unusable (8), med 2 MB-sider
        for i in 0..entries {
            let d = &*(mmap_buf.add(i * desc_size) as *const MemoryDescriptor);
            if d.mem_type == 0 || d.mem_type == 8 {
                continue;
            }
            let start = d.phys_start & !(SIZE_2M - 1);
            let end = (d.phys_start + d.num_pages * 4096 + SIZE_2M - 1) & !(SIZE_2M - 1);
            let mut addr = start;
            while addr < end {
                pt.map_2m(addr, addr, WRITABLE);
                addr += SIZE_2M;
            }
        }

        // Framebufferen står typisk ikke i memory map, så den mappes for sig (RW, ikke eksekverbar)
        let fb_start = mode.frame_buffer_base & !(SIZE_2M - 1);
        let fb_end = (mode.frame_buffer_base + mode.frame_buffer_size as u64 + SIZE_2M - 1)
            & !(SIZE_2M - 1);
        let mut addr = fb_start;
        while addr < fb_end {
            pt.map_2m(addr, addr, WRITABLE | NO_EXECUTE);
            addr += SIZE_2M;
        }

        // Higher half: kernens segmenter med 4 KB-sider og rettigheder fra ELF
        for ph in phdrs {
            if ph.p_type != PT_LOAD {
                continue;
            }
            let mut flags = 0;
            if ph.flags & 2 != 0 {
                flags |= WRITABLE; // PF_W
            }
            if ph.flags & 1 == 0 {
                flags |= NO_EXECUTE; // ikke PF_X
            }
            let mut v = ph.vaddr & !(SIZE_4K - 1);
            while v < ph.vaddr + ph.memsz {
                pt.map_4k(v, kernel_phys + (v - KERNEL_VMA), flags);
                v += SIZE_4K;
            }
        }

        // --- 20. Verificér med en software page walk ---
        // (virtuel adresse, forventet fysisk adresse)
        let here = efi_main as *const () as u64;
        let stack = &mmap_size as *const usize as u64;
        let checks = [
            (KERNEL_VMA, kernel_phys, "PT check: kernel start"),                           // kernens første side
            (elf.entry, kernel_phys + (elf.entry - KERNEL_VMA), "PT check: _start"), // _start
            (here, here, "PT check: loader code"),                                        // loaderens egen kode
            (stack, stack, "PT check: stack"),                                      // stacken
            (fb as u64, fb as u64, "PT check: framebuffer"),                              // framebufferen
        ];
        for (i, &(virt, expected, what)) in checks.iter().enumerate() {
            if pt.translate(virt) != Some(expected) {
                con.fail(18 + i as u64, what);
            }
        }
        let _ = write!(con, "PT: {} tables OK\n", pt.tables);

        // boot services stopper ikke af sig selv; vi hænger bare her.
        loop {}
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
