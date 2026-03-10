[BITS 16] ; Fortæller NASM vi skriver 16-bit kode
[ORG 0x7C00] ; Vores kode bliver loaded på den her adresse af BIOS

start:
  ; 1. Sæt segmentregistre og stak op
  xor ax, ax ; ax = 0
  mov ds, ax ; data segment = 0
  mov es, ax ; extra segment = 0
  mov ss, ax ; stack segment = 0
  mov sp, 0x7C00 ; stakken vokser nedad fra 0x7C00

  ; 2. Load stage 2 fra disk med LBA
  mov si, dap ; peg SI på vores Disk Addres Packet
  mov ah, 0x42 ; BIOS funktion: Extended Read
  mov dl, 0x80 ; disk nummer (0x80 = først harddisk)
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

