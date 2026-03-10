[BITS 16]
[ORG 0x7E00] ; vi blev loadet hetil af stage 1

stage2_start:
  ; print besked så vi ved stage 2 kører
  mov si, msg_stage2
  call print_string_16

  ; Næste skridt: GDT og protected mode (kommer snart som en løve)
  hlt

print_string_16:
  lodsb
  or al, al
  jz .done
  mov ah, 0x0E
  int 0x10
  jmp print_string_16
.done:
  ret

msg_stage2:
  db "Stage 2 loaded!", 0

