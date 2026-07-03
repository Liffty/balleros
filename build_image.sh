#/bin/bash
set -e

# Assemble bootlaoder stages and build kernel
nasm -f bin bootloader/stage1.asm -o stage1.bin
nasm -f bin bootloader/stage2.asm -o stage2.bin
nasm -f elf64 -g -F dwarf -DELF bootloader/stage1.asm -o stage1.elf
nasm -f elf64 -g -F dwarf -DELF bootloader/stage2.asm -o stage2.elf

cargo objcopy --release -- -O binary kernel.bin

# Creat 32MB FAT32 disk image
dd if=/dev/zero of=balleros.img bs=1M count=32
mformat -F -c 8 -i balleros.img ::

# Copy kernelf into FAT32 filesystem FIRST
mcopy -i balleros.img kernel.bin ::KERNEL.BIN

# Write stage1 boot code (preserve mformat's BPB, only cdoe from offset 90+)
dd if=stage1.bin of=balleros.img bs=1 skip=90 seek=90 conv=notrunc

# Write boot signature
printf '\x55\xAA' | dd of=balleros.img bs=1 seek=510 conv=notrunc

# Write stage2 to sector 1+
dd if=stage2.bin of=balleros.img bs=512 seek=1 conv=notrunc
