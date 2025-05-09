ENTRY(_start)

SECTIONS {
    . = 1M;	/* Load ad 1MB address */

    /* Multiboot2 sections */
    .multiboot2 : {
        KEEP(*(.multiboot2))
    }

    /* Other sections */
    .text : { *(.text) }
    .data : { *(.data) }
    .bss  : { *(.bss) }
    /DISCARD/ : { *(.comment) }
}
