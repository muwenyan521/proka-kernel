; Proka Kernel - A kernel for ProkaOS.
; Copyright (C) RainSTR Studio 2025, All Rights Reserved.
;
; This file contains the header of multiboot2, which can
; boot up by using GRUB.

; Define a section "multiboot2"
section .multiboot2
align 8	   ; The alignment required
begin:
    dd 0xE85250D6	; Magic number
    dd 0 		; Architecture, 0 is protected mode
    dd end - begin	; Header length
    dd 0x100000000 - (0xE85250D6 + 0 + (end - begin))	; Checksum

    ; Framebuffer tag
    dw 5        ; Tag type, 5 is framebuffer
    dw 0	; Flags, needed
    dd 20	; Tag size
    dd 1024	; The width
    dd 768	; The height
    dd 32	; The depth

    ; Fill 4 bytes to align
    resb 4

    ; End tag
    dw 0	; Tag type, 0 is end
    dw 0	; Flags
    dd 8	; Tag size
end:

; Stack section
section .bss
align 16
stack_bottom:
    resb 16384 ; 16KB stack, probably enough
stack_top:

section .data
    mb_magic dd 0	; The EAX value storing place (magic num)
    mbi_ptr dd 0	; The EBX value storing place (address)

; The entry of the program
section .text
extern kernel_main
default rel
bits 32
global _start

_start:
    mov [mb_magic], eax	; Save EAX value
    mov [mbi_ptr], ebx 	; Save EBX value
    mov esp, stack_top	; Set up the stack pointer
    
    ; Check for CPUID support
    pushfd
    pop eax
    mov ecx, eax
    xor eax, 0x200000
    push eax
    popfd
    pushfd
    pop eax
    xor eax, ecx
    jz .unsupport_cpu

    ; Check for extended CPUID
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb .unsupport_cpu

    ; Check for Long Mode support
    mov eax, 0x80000001
    cpuid
    test edx, (1 << 29)
    jz .unsupport_cpu
    
    ; Call the kernel entry
    push dword [mb_magic]	; The second argument
    push dword [mbi_ptr]	; The first argument
    call kernel_main		; Call the kernel function
    add esp, 8
     
    hlt

.unsupport_cpu:
    ; Handling Unsupported CPU
    hlt
    jmp .unsupport_cpu

