; Proka Kernel - A kernel for ProkaOS.
; Copyright (C) RainSTR Studio 2025, All Rights Reserved.
;
; This file contains the header of multiboot2, which can
; boot up by using GRUB.

; The data section
section .data
    mbi_ptr dd 0	; The EBX value, which is very important

; The entry of the program
section .text
default rel
extern kernel_main
extern check_cpu
bits 32
global _start
_start:
    mov esp, stack_top
    mov [mbi_ptr], ebx	; push Multiboot info pointer to stack_top
    
    call check_multiboot ; Check is this boot uo by multiboot2

    call check_cpu ; Check for CPUID support
    
    call set_up_page_tables
    call enable_paging
    
    ; 加载64位GDT并跳转
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
set_up_page_tables:
    ; map P4 table recursively
    mov eax, p4_table
    or eax, 0x3 ; present + writable
    mov [p4_table + 511 * 8], eax

    ; map first & 510th P4 entry to P3 table
    mov eax, p3_table
    or eax, 0x3 ; present + writable
    mov [p4_table], eax
    mov [p4_table + 510 * 8], eax

    ; map first P3 entry to P2 table
    mov eax, p2_table
    or eax, 0x3 ; present + writable
    mov [p3_table], eax

    ; map each P2 entry to a huge 2MiB page
    mov ecx, 0         ; counter variable

.map_p2_table:
    ; map ecx-th P2 entry to a huge page that starts at address 2MiB*ecx
    mov eax, 0x200000  ; 2MiB
    mul ecx            ; start address of ecx-th page
    or eax, 0b10000011 ; present + writable + huge
    mov [p2_table + ecx * 8], eax ; map ecx-th entry

    inc ecx            ; increase counter
    cmp ecx, 512       ; if counter == 512, the whole P2 table is mapped
    jne .map_p2_table  ; else map the next entry

    ret
    
enable_paging:
    ; load P4 to cr3 register (cpu uses this to access the P4 table)
    mov eax, p4_table
    mov cr3, eax

    ; enable PAE-flag in cr4 (Physical Address Extension)
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; set the long mode bit & no execute bit in the EFER MSR (model specific register)
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8 | 1 << 11
    wrmsr

    ; enable paging & write protect in the cr0 register
    mov eax, cr0
    or eax, 1 << 31 | 1 << 16
    mov cr0, eax

    ret

check_multiboot:
    cmp eax, 0x36d76289
    jne .no_multiboot
    ret
    
.no_multiboot:
    hlt
    jmp .no_multiboot
    
section .bss
align 4096
p4_table:
    resb 4096
p3_table:
    resb 4096
p2_table:
    resb 4096
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