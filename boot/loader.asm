; Proka Kernel - A kernel for ProkaOS.
; Copyright (C) RainSTR Studio 2025, All Rights Reserved.
;
; This file contains the loader, which will switch to the long
; mode, and jump to the kernel entry.

; The data section
section .data
    mbi_ptr dd 0	; The EBX value, which is very important

; The entry of the program
section .text
default rel
extern set_up_page_tables
extern enable_paging
extern check_cpu
bits 32
global _start
_start:
    mov esp, stack_top	; Initialize the stack pointer
    mov [mbi_ptr], ebx	; Save the EBX value and pass to the kernel later
    
    call check_multiboot ; Check is this boot uo by multiboot2

    call check_cpu ; Check for CPUID support

    call set_up_page_tables
    call enable_paging
    
    ; Load the 64-bit GDT and jump
    lgdt [gdt64.pointer]

    jmp gdt64.code:long_mode_entry

bits 64
extern kernel_main
long_mode_entry:
    ; Update segment registers
    mov ax, 0
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Jump to the 64-bit kernel
    mov edi, [mbi_ptr]	; Pass the argument to the kernel
    jmp kernel_main	; Just jump to it

bits 32
check_multiboot:
    cmp eax, 0x36d76289
    jne .no_multiboot
    ret
    
.no_multiboot:
    hlt
    jmp .no_multiboot
    
section .bss
align 4096
stack_bottom:
    resb 16384 ; 16KB stack, probably enough
stack_top:

section .rodata
gdt64:
    dq 0 ; zero entry
.code: equ $ - gdt64 ; new
    dq (1<<43) | (1<<44) | (1<<47) | (1<<53) ; code segment
.pointer:
    dw $ - gdt64 - 1
    dq gdt64
