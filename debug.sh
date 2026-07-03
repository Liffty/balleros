#!/bin/bash
set -e

./build_image.sh

# -s = åbn GDB-server på TCP::1234
# -S = frys CPU'en indtil debuggeren siger "continue"
qemu-system-x86_64 -drive file=balleros.img,format=raw -no-reboot -s -S -d int 2> /tmp/qemu_debug.log

