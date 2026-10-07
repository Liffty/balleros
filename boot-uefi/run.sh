#!/bin/bash

set -e
cd "$(dirname "$0")"

cargo +nightly build -Z build-std=core,compiler_builtins -Z json-target-spec \
    --target x86_64-balleros-uefi.json

cp target/x86_64-balleros-uefi/debug/bootx64.efi esp/EFI/BOOT/BOOTX64.EFI

qemu-system-x86_64 \
    -drive if=pflash,format=raw,readonly=on,file="$(brew --prefix qemu)/share/qemu/edk2-x86_64-code.fd" \
    -drive format=raw,file=fat:rw:esp \
    -net none
