# balleros
BallerOS is a hobby Operating System.

Why have i set out on this journey? No clue, just found it interesting.

Right now the stage i still being set. I have decided to write my own bootloader.
This will properly take a long time, so to call this an OS project, will be kind of misleading.

Inspiration: "Fuck it, we ball" and "Ball is life"

## Journal entry

### Journal entry 12-03-2026

As of now, the bootloader boots from BIOS 16-bit real mode. Loads stage 2 via LBA. Set up GDT and switch to 32-bit protected. I set up 4 levels of page tables. Switch to 64-bit long mode. Read the kernel from disk via ATA PIO-driver. Jump to the Rust kernel. 



