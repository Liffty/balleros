// x86_64 4-niveau paging: PML4 -> PDPT -> PD -> PT
// Hver tabel er én 4 KB-side med 512 indgange á 8 bytes.
// Mens vi bygger, kører vi stadig på firmwarens identity mapping,
// så en tabels fysiske adresse kan bruges direkte som pointer.

pub const PRESENT: u64 = 1 << 0;
pub const WRITABLE: u64 = 1 << 1;
pub const HUGE: u64 = 1 << 7; // i en PD-indgang: 2 MB-side i stedet for pointer til en PT
pub const NO_EXECUTE: u64 = 1 << 63;

const ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000; // bit 12-51: fysisk adresse

pub const SIZE_4K: u64 = 0x1000;
pub const SIZE_2M: u64 = 0x20_0000;

// Indeks i tabellen på et givent niveau for en virtuel adresse.
// 4 = PML4 (bit 39-47), 3 = PDPT (bit 30-38), 2 = PD (bit 21-29), 1 = PT (bit 12-20)
fn index(virt: u64, level: u32) -> usize {
    ((virt >> (12 + 9 * (level - 1))) & 0x1FF) as usize
}

pub struct PageTables<F: FnMut() -> u64> {
    pub pml4: u64,     // fysisk adresse på PML4 - det CR3 skal pege på
    pub tables: usize, // antal allokerede tabeller (til debug-print)
    alloc: F,          // giver en nulstillet 4 KB-side
}

impl<F: FnMut() -> u64> PageTables<F> {
    pub fn new(mut alloc: F) -> Self {
        let pml4 = alloc();
        PageTables { pml4, tables: 1, alloc }
    }

    // Find (eller opret) den tabel som indgang `idx` i `table` peger på.
    unsafe fn next_table(&mut self, table: u64, idx: usize) -> u64 {
        unsafe {
            let entry = (table as *mut u64).add(idx);
            if *entry & PRESENT == 0 {
                let new = (self.alloc)();
                self.tables += 1;
                // Mellemniveauer er altid RW og eksekverbare.
                // Rettighederne bestemmes af den nederste indgang.
                *entry = new | PRESENT | WRITABLE;
            }
            *entry & ADDR_MASK
        }
    }

    // Map én 2 MB-side. Springes over hvis den allerede er mappet
    // (flere memory map-indgange kan dele den samme 2 MB-blok).
    pub unsafe fn map_2m(&mut self, virt: u64, phys: u64, flags: u64) {
        unsafe {
            let pdpt = self.next_table(self.pml4, index(virt, 4));
            let pd = self.next_table(pdpt, index(virt, 3));
            let entry = (pd as *mut u64).add(index(virt, 2));
            if *entry & PRESENT == 0 {
                *entry = phys | flags | PRESENT | HUGE;
            }
        }
    }

    // Map én 4 KB-side.
    pub unsafe fn map_4k(&mut self, virt: u64, phys: u64, flags: u64) {
        unsafe {
            let pdpt = self.next_table(self.pml4, index(virt, 4));
            let pd = self.next_table(pdpt, index(virt, 3));
            let pt = self.next_table(pd, index(virt, 2));
            *(pt as *mut u64).add(index(virt, 1)) = phys | flags | PRESENT;
        }
    }

    // Software page walk - gør det samme som CPU'en. Bruges til at verificere tabellerne.
    pub unsafe fn translate(&self, virt: u64) -> Option<u64> {
        unsafe {
            let mut table = self.pml4;
            for level in (1..=4).rev() {
                let entry = *(table as *const u64).add(index(virt, level));
                if entry & PRESENT == 0 {
                    return None;
                }
                if level == 2 && entry & HUGE != 0 {
                    return Some((entry & ADDR_MASK) + (virt & (SIZE_2M - 1)));
                }
                table = entry & ADDR_MASK;
            }
            Some(table + (virt & (SIZE_4K - 1)))
        }
    }
}
