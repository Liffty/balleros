[BITS 16] ; Fortæller NASM vi skriver 16-bit kode
[ORG 0x7C00] ; Vores kode bliver loaded på den her adresse af BIOS


; === The first 3 bytes: jump over BPB ===
  jmp short start
  nop


; ================
; BPB (BIOS Parameter block) - this is describing the FAT32-filsystem
; Theese values will match the formated disk-image
; We will use placholder values now ; same oas the line above???, and we will overwrite them. 
; This is because when we dd this will overwrite, so we have to jump over it.

bpb_oem_id: db "BALLEROS" ; 8 bytes OEM identifikation.
bpb_bytes_per_sec: dw 512 ; Bytes per sektor
bpb_sec_per_clus: dw 8 ; sector per cluster
bpb_reserved_secs_ dw 32 ; reserverd sector (should be space enogugh for stage2)
bpb_num_fats: db 2 ; number of FAT-copys
bpb_root_ent_cnt: dw 0 ; 0 for FAT32 (root dir is in clusters)
bpb_total_sec_16: dw 0 ; 0 for FAT32
bpb_media_type: db 0xF8 ; harddisk
bpb_fat_size_16: dw 0 ; 0 for FAT32
bpb_sec_per_track: dw 63 ; geometri (ingored with LBA) dont understand why we have to write this?
bpb_num_heads: dw 255 ; same oas the line above???
bpb_hidden_secs: dd 0 ; number of hidden sectors
bpb_total_sec_32 dd 65536 ; total number of sectors (32 MB / 512)

; FAT32 Extended BPB
bpb_fat_size_32: dd 512 ; sector per FAT
bpb_ext_flags: dw 0 ; flags
bpb_fs_version: dw 0 ; FAT32 version (0.0)
bpb_root_cluster: dd 1 ; root directory starts in custer 2
bpb_backup_boot: dw 6 ; backup root sector
bpb_reserved: times 12 db 0; reserverd
bpb_drive_num: db 0x80 ; harddisk
bpb_reserved2: db 0 ; reserverd
bpb_boot_sig: db 0x29 ; extended boot signatur
bpb_volume_id: dd 0x12345678 ; volume serial number
bpb_volume_label dd "BALLEROS   " ; 11 bytes volume label
bpb_fs_type: db "FAT32  " ; 8 bytes filesystem.


start:
  ; 1. Sæt segmentregistre og stak op
  xor ax, ax ; ax = 0
  mov ds, ax ; data segment = 0
  mov es, ax ; extra segment = 0
  mov ss, ax ; stack segment = 0
  mov sp, 0x7C00 ; stakken vokser nedad fra 0x7C00

  ; save the boot drive number (BIOS gives os that i DL)
  mov [boot_drive], dl

  ; 2. Load stage 2 fra disk med LBA
  mov si, dap ; peg SI på vores Disk Addres Packet
  mov ah, 0x42 ; BIOS funktion: Extended Read
  mov dl, [boot_drive] ; Now we use dl to get the boot drive number.
  int 0x13 ; Kald BIOS
  jc disk_error ; hvis carry flag er sat, fejlede læsningen

  ; 3. Hop til stage 2
  jmp 0x0000:0x7E00

disk_error:
  mov si, error_msg
  call print_string
  hlt ; stop CPU

print_string:
  lodsb ; load næste byte fra SI ind i AL
  or al, al ; er det null-terminatoren?
  jz .done
  mov ah, 0x0E ; BIOS funktion: print tegn
  int 0x10 ; kald BIOS video interrupt
  jmp print_string
.done:
  ret


; Data
boot_drive: db 0

; Disk Addres Packet (DAP) - fortæller BIOS hvad vi vil læse
dap:
  db 0x10 ; størrelsen af pakken (16 bytes)
  db 0 ; reserveret altid 0
  dw 4 ; antal sektorer at læse (4 = 2KB)
  dw 0x7E00 ; offset - load hertil
  dw 0x0000 ; segment
  dq 1  ; LBA startnummer (sektot 1, dvs. lige efter boot sector)

error_msg:
  db "Disk read error!", 0

; 4. Fyld resten med nuller og afslut med boot-signitur
times 510 - ($ - $$) db 0 ; pad til 510 bytes
dw 0xAA55 ; boot signatur (little-endian)

