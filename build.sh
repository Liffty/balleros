#!/bin/bash
set -e

./build_image.sh

qemu-system-x86_64 -drive file=balleros.img,format=raw -no-reboot -d int 2> /tmp/qemu_debug.log