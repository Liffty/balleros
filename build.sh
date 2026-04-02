#!/bin/bash
set -e

# Assemble bootloader stages and build kernel
nasm -f bin bootloader/stage1.asm -o stage1.bin
nasm -f bin bootloader/stage2.asm -o stage2.bin
cargo objcopy --release -- -O binary kernel.bin

# Create 32MB FAT32 disk image
dd if=/dev/zero of=balleros.img bs=1M count=32
mformat -F -c 8 -i balleros.img ::

# Copy kernel into FAT32 filesystem FIRST (before overwriting bootloader sectors)
mcopy -i balleros.img kernel.bin ::KERNEL.BIN

# Write stage1 boot code (preserve mformat's BPB, only code from offset 90+)
dd if=stage1.bin of=balleros.img bs=1 skip=90 seek=90 conv=notrunc

# Write boot signature
printf '\x55\xAA' | dd of=balleros.img bs=1 seek=510 conv=notrunc

# Write stage2 to sector 1+ (overwrites FSInfo, but that's OK)
dd if=stage2.bin of=balleros.img bs=512 seek=1 conv=notrunc

# Run
qemu-system-x86_64 -drive file=balleros.img,format=raw -no-reboot -d int 2> /tmp/qemu_debug.log
