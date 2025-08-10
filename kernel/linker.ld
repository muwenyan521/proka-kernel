/* The output file type */
OUTPUT_FORMAT("elf64-x86-64")

/* The entry point */
ENTRY(kernel_main)

/* The program header */
PHDRS {
    text PT_LOAD;
    data PT_LOAD;
    rodata PT_LOAD;
    bss PT_LOAD;
}

/* The section define area */
SECTIONS {
    . = 0xffffffff80000000;	/* Load at high-half address */

    /* Code (text) sections */
    .text : {
        *(.text .text.*)
    } :text

    /* Move to the next memory page */
    . = ALIGN(CONSTANT(MAXPAGESIZE));

    /* Read-only data (rodata) sections */
    .rodata : {
        *(.rodata .rodata.*)
    } :rodata

    /* Move to the next memory page */
    . = ALIGN(CONSTANT(MAXPAGESIZE));

    /* Data sections */
    .data : {
        *(.data .data.*)

        /* Place the sections that contains the limine request */
        KEEP(*(.requests_start_marker))
        KEEP(*(.requests))
        KEEP(*(.requests_end_marker))
    } :data

    /* Move to the next memory page */
    . = ALIGN(CONSTANT(MAXPAGESIZE));

    /* BSS sections */
    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    } :bss

    /* Something we want to discard */
    /DISCARD/ : {
        *(.comment .comment.*)
        *(.note .note.*)
        *(.eh_frame .eh_frame.*)
    }
}
