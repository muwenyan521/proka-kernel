ENTRY(_start)

SECTIONS {
    . = 1M;	/* Load at 1MB address */

    /* Multiboot2 sections */
    .multiboot2 : {
        KEEP(*(.multiboot2))
    }

    /* Font data sections */
    .font_data : ALIGN(4K) {
        KEEP(*(.font_data*))
    }

    /* Other sections */
    .rodata : ALIGN(4K) { *(.rodata*) }
    .text : ALIGN(4K) { *(.text) }
    .data : ALIGN(4K) { *(.data) }
    .bss  : ALIGN(4K) { *(.bss) }
    /DISCARD/ : { *(.comment) }
}
