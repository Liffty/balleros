[BITS 16]
[ORG 0x7E00] ; vi blev loadet hetil af stage 1

stage2_start:
  ; print besked så vi ved stage 2 kører
  mov si, msg_stage2
  call print_string_16

  ; === We prepare the transformation to 32-bit protected mode ===
  cli  ; Turn off interrupts we will not want BIOS 
       ; interrupts in the middle of mode change
  
  lgdt [gdt_descriptor] ; Load the GDT in the CPU's GDTR-register

  ; We turn on protected mode: put bit 0 i CR0
  mov eax, cr0
  or eax, 1
  mov cr0, eax
  
  ; Far jump to 32-bit code, this should flush the CPU pipeline
  ; 0x80 is the offset to our code segment in the GDT (entry 1 x 8 bytes)
  jmp 0x08:protected_mode_start


; === Print function (16 bit, used before mode change) ===
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
; Every entry is 8 bytes, and that describes a memorysegment
; The format is fucked because whoever made it, wanted pain

gdt_start:

; Entry 0: Null decripter (The cpu demands this)
gdt_null:
  dq 0  ; 8 bytes of zeros

; Entry 1: code segment
; Base = 0x00000000, Limit = 0xFFFFF (with granularity = 4KB -> covers 4 GB)
; Flags: executable, readable, 32-bit, present
gdt_code:
  dw 0xFFFF ; Limit bits 0-15
  dw 0x0000 ; Base bits 0-15
  db 0x00 ; Base bits 16-23
  db 10011010b  ; Access byte:
  ; 1 = present
  ; 00 = ring 0 highest kernel privilages
  ; 1 code/data segment
  ; 1 yes it is executalbe
  ; 0 conforming
  ; 1 readable
  ; 0 accessed by CPU
  db 11001111b  ; Flags + limit bits 16-19:
  ; 1 granularity (limit x 4KB)
  ; 1 32-bit segment
  ; 00 reserved
  ; 1111 limit bits 16-19
  db 0x00 ; Base bits 24-31

; Entry 2: Data segment
; Same as the code, but this is writeble instead of executable
gdt_data:
  dw 0xFFFF
  dw 0x0000
  db 0x00
  db 10010010b ; Access byte
  ; 1 present
  ; 00 ring 0
  ; 1 code/data
  ; 0 NOT executable (data segment)
  ; 0 expand-up
  ; 1 writeble
  ; 0 accessed


  db 11001111b
  db 0x00
gdt_end:

; GDT descripter - tells the CPU where the GDT is, and how big it is
gdt_descriptor:
  dw gdt_end - gdt_start - 1 ; sizr (number of bytes minus 1)
  dd gdt_start ; Address on the GDT

; ========
; 32-bit Protected Mode
; ========
[BITS 32]

protected_mode_start:
  ; Update segmentregisters so that is uses our data segments
  ; 0x10 = offset to the data segment i GDT (entry 2 x 8 bytes)
  mov ax, 0x10
  mov ds, ax
  mov es, ax 
  mov fs, ax
  mov gs, ax
  mov ss, ax
  mov esp, 0x90000 ; New stack, we need a 32-bit stack

  ; We can write this to our VGA text buffer as proof
  ; We can't use the nice function that BIOS gave us anymore
  ; We know the exact address of the VGA text buffer 0xB8000
  ; Every character is 2 bytes: [ASCII-character] [Color]
  mov edi, 0xB8000
  mov esi, msg_pmode
  mov ah, 0x0A ; Color: light green on black

.print_loop:
  lodsb
  or al, al
  jz .done
  mov [edi], ax ; write character plus the color
  add edi, 2  ; Next position (2 bytes per character)
  jmp .print_loop

.done:
  ; === initialize page tabels (4 levels, 4KB pages) ===
  ;  reset the mememory for page tables (4 x 4096 bytes)
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

  ; Fyldt PT med 512 entries der mapper 0x0000 - 0x1FFFFF (2 MB)
  mov edi, 0x4000
  mov eax, 0x0003
  mov ecx, 512

.fill_pt:
  mov[edi], eax
  add eax, 0x1000
  add edi, 8
  loop .fill_pt

  ; Page tabels should be ready
  ; idk tho
  
  ; === Activate long mode ===
  ; Turn on PEA (Physical Address Extension) in CR4
  mov eax, cr4
  or eax, 1 << 5 ; place bit 5 (PEA)
  mov cr4, eax

  ; Activate long mode in EFER MSR (Model specific Register)
  mov ecx, 0xC0000080 ; EFER register-number
  rdmsr ; Read EFER into EAX
  or eax, 1 << 8 ; set bit 8 (Long mode enable)
  wrmsr ; Write back

  ; Turn on paging i CR0 (bit 31)
  mov eax, cr0
  or eax, 1 << 31
  mov cr0, eax

  ; Load new 64-GDT
  lgdt [gdt64_descripter]

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
  dw 0xFFFF ; Limit (ingored in long mode, i think)
  dw 0x0000 ; Base
  db 0x00 ; Base
  db 10011010b ; present, ring 0, executable, readable
  db 10101111b ; 64-bit flag (bit 5) + granularity + limit
  db 0x00 ; Base

; 64-bit data segment
gdt64_data:
  dw 0xFFFF
  dw 0x0000
  db 0x00
  db 10010010b ; presentm ring 0, writable
  db 10101111b
  db 0x00

gdt64_end:

gdt64_descripter:
  dw gdt64_end - gdt64_start - 1
  dd gdt64_start


; =======
; 64-bit Long mode
; =======

[BITS 64]

long_mode_start:
  ; Update segmentregisters
  mov ax, 0x10 ; data segment (entry 2 in GDT64)
  mov ds, ax
  mov es, ax
  mov fs, ax
  mov gs, ax
  mov ss, ax

  ; Write to VGA buffer as proof that we are now in 64 - bit (baller)
  mov rdi, 0xB8000
  add rdi, 160 ; line 2 (80 characters x 2 bytes)
  mov rsi, msg_long
  mov ah, 0x0E ; color, yellow on black

.print64:
  lodsb
  or al, al
  jz .done64
  mov [rdi], ax
  add rdi, 2
  jmp .print64

.done64:
  hlt

msg_long:
  db "64-bit mode baller",0


msg_stage2:
  db "Stage 2 loaded!, Switching to protected mode...", 0

msg_pmode:
  db "32-bit protected mode!!!!!"
