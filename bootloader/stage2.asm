[BITS 16]
[ORG 0x7E00] ; loaded here by stage 1

stage2_start:
  ; Print message so we know stage 2 is running
  mov si, msg_stage2
  call print_string_16

  ; === Read BPB from boot sector (0x7C00) ===
  ; The boot sector is still in memory from BIOS
  ;
  ; Calculate start of data area:
  ; data_start_sector = reserved_sectors + (num_fats * fat_size_32)

  xor eax, eax
  mov ax, [0x7C00 + 14] ; BPB offset 14 = reserved_sectors (word)
  mov [bpb_reserved], ax

  xor eax, eax
  mov al, [0x7C00 + 16] ; BPB offset 16 = num_fats (byte)
  mov [bpb_nfats], al

  mov eax, [0x7C00 + 36] ; BPB offset 36 = fat_size_32 (dword)
  mov [bpb_fatsize], eax

  xor eax, eax
  mov al, [0x7C00 + 13] ; BPB offset 13 = sectors_per_cluster (byte)
  mov [bpb_spc], al

  mov eax, [0x7C00 + 44] ; BPB offset 44 = root_cluster (dword)
  mov [bpb_rootclus], eax

  ; calculate data_start = reserved + (num_fats * fat_size)
  xor eax, eax
  mov al, [bpb_nfats]
  mul dword [bpb_fatsize] ; eax = num_fats * fat_size
  xor ebx, ebx
  mov bx, [bpb_reserved]
  add eax, ebx            ; eax = reserved + (num_fats * fat_size)
  mov [data_start_sector], eax

  ; calculate fat_start = reserved_sectors
  xor eax, eax
  mov ax, [bpb_reserved]
  mov [fat_start_sector], eax

  ; === Prepare the transition to 32-bit protected mode ===
  cli  ; Disable interrupts during mode change

  lgdt [gdt_descriptor] ; Load the GDT into the CPU's GDTR register

  ; Enable protected mode: set bit 0 in CR0
  mov eax, cr0
  or eax, 1
  mov cr0, eax

  ; Far jump to 32-bit code - flushes the CPU pipeline
  ; 0x08 is the offset to our code segment in the GDT (entry 1 x 8 bytes)
  jmp 0x08:protected_mode_start


; === Print function (16-bit, used before mode change) ===
print_string_16:
  lodsb
  or al, al
  jz .done
  mov ah, 0x0E
  int 0x10
  jmp print_string_16
.done:
  ret


; ============
; GDT (Global Descriptor Table)
; ============
; Each entry is 8 bytes and describes a memory segment.
; The byte layout is non-linear for historical x86 reasons.

gdt_start:

; Entry 0: Null descriptor (required by the CPU)
gdt_null:
  dq 0  ; 8 bytes of zeros

; Entry 1: Code segment
; Base = 0x00000000, Limit = 0xFFFFF (with granularity = 4KB -> covers 4 GB)
; Flags: executable, readable, 32-bit, present
gdt_code:
  dw 0xFFFF ; Limit bits 0-15
  dw 0x0000 ; Base bits 0-15
  db 0x00   ; Base bits 16-23
  db 10011010b  ; Access byte:
  ; 1 = present
  ; 00 = ring 0 (highest kernel privileges)
  ; 1 = code/data segment
  ; 1 = executable
  ; 0 = conforming
  ; 1 = readable
  ; 0 = accessed by CPU
  db 11001111b  ; Flags + limit bits 16-19:
  ; 1 = granularity (limit x 4KB)
  ; 1 = 32-bit segment
  ; 00 = reserved
  ; 1111 = limit bits 16-19
  db 0x00   ; Base bits 24-31

; Entry 2: Data segment
; Same as code, but writable instead of executable
gdt_data:
  dw 0xFFFF
  dw 0x0000
  db 0x00
  db 10010010b ; Access byte:
  ; 1 = present
  ; 00 = ring 0
  ; 1 = code/data
  ; 0 = NOT executable (data segment)
  ; 0 = expand-up
  ; 1 = writable
  ; 0 = accessed
  db 11001111b
  db 0x00
gdt_end:

; GDT descriptor - tells the CPU where the GDT is and how big it is
gdt_descriptor:
  dw gdt_end - gdt_start - 1 ; size (number of bytes minus 1)
  dd gdt_start                ; address of the GDT

; ========
; 32-bit Protected Mode
; ========
[BITS 32]

protected_mode_start:
  ; Update segment registers to use our data segment
  ; 0x10 = offset to the data segment in GDT (entry 2 x 8 bytes)
  mov ax, 0x10
  mov ds, ax
  mov es, ax
  mov fs, ax
  mov gs, ax
  mov ss, ax
  mov esp, 0x90000 ; new 32-bit stack

  ; Write to VGA text buffer as proof of protected mode
  ; VGA text buffer is at 0xB8000
  ; Each character is 2 bytes: [ASCII char] [color attribute]
  mov edi, 0xB8000
  mov esi, msg_pmode
  mov ah, 0x0A ; color: light green on black

.print_loop:
  lodsb
  or al, al
  jz .done
  mov [edi], ax ; write character + color
  add edi, 2    ; next position (2 bytes per character)
  jmp .print_loop

.done:
  ; === Initialize page tables (4 levels, 4KB pages) ===
  ; Clear memory for page tables (4 x 4096 bytes)
  mov edi, 0x1000
  mov cr3, edi
  xor eax, eax
  mov ecx, 4096
  rep stosd

  ; PML4 entry 0 -> PDPT
  mov dword [0x1000], 0x2003

  ; PML3 entry 0 -> PDT
  mov dword [0x2000], 0x3003

  ; PML2 entry 0 -> PT
  mov dword [0x3000], 0x4003

  ; Fill PT with 512 entries mapping 0x0000 - 0x1FFFFF (2 MB)
  mov edi, 0x4000
  mov eax, 0x0003
  mov ecx, 512

.fill_pt:
  mov [edi], eax
  add eax, 0x1000
  add edi, 8
  loop .fill_pt

  ; === Activate long mode ===
  ; Enable PAE (Physical Address Extension) in CR4
  mov eax, cr4
  or eax, 1 << 5 ; set bit 5 (PAE)
  mov cr4, eax

  ; Enable long mode in EFER MSR (Model Specific Register)
  mov ecx, 0xC0000080 ; EFER register number
  rdmsr               ; read EFER into EAX
  or eax, 1 << 8      ; set bit 8 (Long Mode Enable)
  wrmsr               ; write back

  ; Enable paging in CR0 (bit 31)
  mov eax, cr0
  or eax, 1 << 31
  mov cr0, eax

  ; Load 64-bit GDT
  lgdt [gdt64_descriptor]

  ; Far jump to 64-bit code
  jmp 0x08:long_mode_start


; ======
; 64-bit GDT
; ======
gdt64_start:

gdt64_null:
  dq 0

; 64-bit code segment
gdt64_code:
  dw 0xFFFF     ; Limit (ignored in long mode)
  dw 0x0000     ; Base
  db 0x00       ; Base
  db 10011010b  ; present, ring 0, executable, readable
  db 10101111b  ; 64-bit flag (bit 5) + granularity + limit
  db 0x00       ; Base

; 64-bit data segment
gdt64_data:
  dw 0xFFFF
  dw 0x0000
  db 0x00
  db 10010010b  ; present, ring 0, writable
  db 10101111b
  db 0x00

gdt64_end:

gdt64_descriptor:
  dw gdt64_end - gdt64_start - 1
  dd gdt64_start


; =======
; 64-bit Long Mode
; =======

[BITS 64]

long_mode_start:
  ; Update segment registers
  mov ax, 0x10 ; data segment (entry 2 in GDT64)
  mov ds, ax
  mov es, ax
  mov fs, ax
  mov gs, ax
  mov ss, ax

  ; Write to VGA buffer as proof that we are now in 64-bit (baller)
  mov rdi, 0xB8000
  add rdi, 160   ; line 2 (80 characters x 2 bytes)
  mov rsi, msg_long
  mov ah, 0x0E   ; color: yellow on black

.print64:
  lodsb
  or al, al
  jz .done64
  mov [rdi], ax
  add rdi, 2
  jmp .print64

.done64:
  ; === Find and load KERNEL.BIN from FAT32 ===

  ; Calculate root directory sector:
  ; sector = data_start + (root_cluster - 2) * sectors_per_cluster
  xor rax, rax
  mov eax, [bpb_rootclus]
  sub eax, 2
  xor rbx, rbx
  mov bl, [bpb_spc]
  mul ebx
  xor rbx, rbx
  mov ebx, [data_start_sector]
  add rax, rbx

  ; Load the root directory to 0x80000 (temporary buffer)
  mov rdi, 0x80000
  mov rsi, rax        ; start sector
  xor rcx, rcx
  mov cl, [bpb_spc]   ; read one cluster
  call ata_read_sectors

  ; Search for KERNEL.BIN in directory entries
  mov rdi, 0x80000    ; start of root dir buffer
  mov rcx, 16         ; max entries to check

.search_entry:
  ; Check if entry is empty (end of directory)
  mov al, [rdi]
  cmp al, 0x00
  je .kernel_not_found

  ; Check if entry has been deleted
  cmp al, 0xE5
  je .next_entry

  ; Compare 11 bytes with "KERNEL  BIN" (8.3 format)
  push rcx
  push rdi
  mov rsi, kernel_filename
  mov rcx, 11
  repe cmpsb
  pop rdi
  pop rcx
  je .kernel_found

.next_entry:
  add rdi, 32  ; next directory entry (32 bytes each)
  loop .search_entry

.kernel_not_found:
  ; Print error message via VGA buffer
  mov rdi, 0xB8000
  add rdi, 320       ; line 3
  mov rsi, msg_no_kernel
  mov ah, 0x0C       ; color: red

.print_err:
  lodsb
  or al, al
  jz .err_done
  mov [rdi], ax
  add rdi, 2
  jmp .print_err
.err_done:
  hlt

.kernel_found:
  ; Get the start cluster from the directory entry
  ; cluster high = offset 20, cluster low = offset 26
  xor rax, rax
  mov ax, [rdi + 20]  ; cluster high
  shl eax, 16
  mov ax, [rdi + 26]  ; cluster low
  mov [kernel_cluster], eax

  ; Get the file size
  mov eax, [rdi + 28]
  mov [kernel_size], eax

  ; Calculate how many sectors to read: (size + 511) / 512
  add eax, 511
  shr eax, 9           ; divide by 512
  mov [kernel_sectors], eax

  ; Calculate the kernel's disk sector:
  ; sector = data_start + (cluster - 2) * sectors_per_cluster
  xor rax, rax
  mov eax, [kernel_cluster]
  sub eax, 2
  xor rbx, rbx
  mov bl, [bpb_spc]
  mul ebx
  xor rbx, rbx
  mov ebx, [data_start_sector]
  add rax, rbx

  ; Load the kernel to 0x100000 (1 MB)
  mov rdi, 0x100000
  mov rsi, rax
  xor rcx, rcx
  mov ecx, [kernel_sectors]
  call ata_read_sectors

  ; Jump to kernel
  jmp 0x100000


; ==============
; ATA PIO Disk Driver (64-bit)
; ==============
; Input:
;   RDI = destination address in memory
;   RSI = LBA start sector
;   RCX = number of sectors to read

ata_read_sectors:
  push rcx  ; save sector count

.read_next_sector:
  call ata_wait_ready

  ; Tell the controller what to read
  mov dx, 0x1F2       ; sector count port
  mov al, 1           ; read one sector
  out dx, al

  mov dx, 0x1F3       ; LBA low (bits 0-7)
  mov rax, rsi
  out dx, al

  mov dx, 0x1F4       ; LBA mid (bits 8-15)
  shr rax, 8
  out dx, al

  mov dx, 0x1F5       ; LBA high (bits 16-23)
  shr rax, 8
  out dx, al

  mov dx, 0x1F6       ; Drive/Head + LBA bits 24-27
  shr rax, 8
  and al, 0x0F        ; keep only lower 4 bits
  or al, 0xE0         ; bit 7=1, bit 6=LBA, bit 5=1, bit 4=0 (master)
  out dx, al

  mov dx, 0x1F7       ; command port
  mov al, 0x20        ; command: Read Sectors
  out dx, al

  call ata_wait_data

  ; Read 256 words (512 bytes = 1 sector) from data port
  mov dx, 0x1F0
  mov rcx, 256
  rep insw             ; read word from port DX to [RDI], repeat RCX times

  ; Next sector (RDI already advanced by rep insw)
  inc rsi

  pop rcx              ; restore sector count
  dec rcx
  jz .done
  push rcx
  jmp .read_next_sector

.done:
  ret

; Wait until disk is ready (BSY bit cleared)
ata_wait_ready:
  mov dx, 0x1F7
.wait:
  in al, dx
  test al, 0x80        ; bit 7 = BSY
  jnz .wait
  ret

; Wait until data is ready (DRQ bit set)
ata_wait_data:
  mov dx, 0x1F7
.wait_drq:
  in al, dx
  test al, 0x08        ; bit 3 = DRQ
  jz .wait_drq
  ret

; === String data ===
msg_long:     db "64-bit mode baller", 0
msg_stage2:   db "Stage 2 loaded!, Switching to protected mode...", 0
msg_pmode:    db "32-bit protected mode!!!!!"

; === FAT32 kernel loading data ===
kernel_filename: db "KERNEL  BIN" ; 8.3 format: 8 bytes name + 3 bytes ext
kernel_cluster:  dd 0
kernel_size:     dd 0
kernel_sectors:  dd 0
msg_no_kernel:   db "KERNEL.BIN not found!", 0

; === Cached BPB values (filled at boot from sector 0) ===
bpb_reserved:       dw 0
bpb_nfats:          db 0
bpb_fatsize:        dd 0
bpb_spc:            db 0
bpb_rootclus:       dd 0
data_start_sector:  dd 0
fat_start_sector:   dd 0
