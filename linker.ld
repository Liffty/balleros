/* Entry-punkt: symbolet bootloaderen forventer at hoppe til */
ENTRY(_start)

SECTIONS
{
    /* Kernen loades til 1 MB, præcis hvor stage2's `jmp 0x100000` peger hen */
    . = 0x100000;

    __kernel_start = .;

    /* .text.entry tvinges allerførst, så _start ligger på 0x100000.
       Derefter resten af koden. */
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    /* Read-only data: konstanter, streng-literaler */
    .rodata : {
        *(.rodata .rodata.*)
    }

    /* Initialiseret, skrivbar data */
    .data : {
        *(.data .data.*)
    }

    /* Uinitialiseret data (nul ved opstart).
       Grænsesymbolerne bruges senere til at nulstille .bss. */
    __bss_start = .;
    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }
    __bss_end = .;

    __kernel_end = .;

    /* Smid linker-metadata væk vi ikke bruger i en bare-metal binær */
    /DISCARD/ : {
        *(.comment)
        *(.eh_frame)
        *(.note .note.*)
    }
}
