[BITS 16] ; 16-bit real mode code
[ORG 0x7C00] ; BIOS loads the boot sector at this address

; === The first 3 bytes: jump over BPB ===
  jmp short start
  nop

; ================
; BPB (BIOS Parameter Block) - describes the FAT32 filesystem
; These are placeholder values. The build script preserves mformat's BPB
; (bytes 0-89) and only copies our boot code (bytes 90+) into the image.
; We still need correct sizes here so the assembler places code at offset 90.

bpb_oem_id:        db "BALLEROS" ; 8 bytes OEM identification
bpb_bytes_per_sec: dw 512   ; bytes per sector
bpb_sec_per_clus:  db 8     ; sectors per cluster
bpb_reserved_secs: dw 32    ; reserved sectors (space for stage2)
bpb_num_fats:      db 2     ; number of FAT copies
bpb_root_ent_cnt:  dw 0     ; 0 for FAT32 (root dir is in clusters)
bpb_total_sec_16:  dw 0     ; 0 for FAT32
bpb_media_type:    db 0xF8  ; hard disk
bpb_fat_size_16:   dw 0     ; 0 for FAT32
bpb_sec_per_track: dw 63    ; CHS geometry (legacy, ignored with LBA)
bpb_num_heads:     dw 255   ; CHS geometry (legacy, required by some BIOSes)
bpb_hidden_secs:   dd 0     ; number of hidden sectors
bpb_total_sec_32:  dd 65536 ; total number of sectors (32 MB / 512)

; FAT32 Extended BPB
bpb_fat_size_32:   dd 512   ; sectors per FAT
bpb_ext_flags:     dw 0     ; flags
bpb_fs_version:    dw 0     ; FAT32 version (0.0)
bpb_root_cluster:  dd 2     ; root directory starts in cluster 2
bpb_fsinfo:        dw 1     ; FSInfo sector number
bpb_backup_boot:   dw 6     ; backup boot sector
bpb_reserved:      times 12 db 0 ; reserved
bpb_drive_num:     db 0x80  ; hard disk
bpb_reserved2:     db 0     ; reserved
bpb_boot_sig:      db 0x29  ; extended boot signature
bpb_volume_id:     dd 0x12345678  ; volume serial number
bpb_volume_label:  db "BALLEROS   " ; 11 bytes volume label
bpb_fs_type:       db "FAT32   "    ; 8 bytes filesystem type

start:
  ; Set up segment registers and stack
  xor ax, ax   ; ax = 0
  mov ds, ax   ; data segment = 0
  mov es, ax   ; extra segment = 0
  mov ss, ax   ; stack segment = 0
  mov sp, 0x7C00 ; stack grows downward from 0x7C00

  ; Save the boot drive number (BIOS passes it in DL)
  mov [boot_drive], dl

  ; Load stage 2 from disk using LBA
  mov si, dap    ; point SI to our Disk Address Packet
  mov ah, 0x42   ; BIOS function: Extended Read
  mov dl, [boot_drive]
  int 0x13       ; call BIOS
  jc disk_error  ; if carry flag is set, read failed

  ; Jump to stage 2
  jmp 0x0000:0x7E00

disk_error:
  mov si, error_msg
  call print_string
  hlt

print_string:
  lodsb          ; load next byte from SI into AL
  or al, al     ; is it the null terminator?
  jz .done
  mov ah, 0x0E  ; BIOS function: teletype output
  int 0x10      ; call BIOS video interrupt
  jmp print_string
.done:
  ret

; Data
boot_drive: db 0

; Disk Address Packet (DAP) - tells BIOS what to read
dap:
  db 0x10   ; packet size (16 bytes)
  db 0      ; reserved, always 0
  dw 4      ; number of sectors to read (4 = 2KB)
  dw 0x7E00 ; offset - load to this address
  dw 0x0000 ; segment
  dq 1      ; LBA start sector (sector 1, right after boot sector)

error_msg:
  db "Disk read error!", 0

; Pad to 510 bytes and end with boot signature
times 510 - ($ - $$) db 0
dw 0xAA55 ; boot signature (little-endian)
