; Proka Kernel - A kernel for ProkaOS.
; Copyright (C) RainSTR Studio 2025, All Rights Reserved.
;
; This file contains the checker of CPU, which will check up the
; CPUID, Extention CPUID and Long mode support.
;
; If one is not passed, it won't run anymore 
; (the check_cpu.unsupported_cpu will handle it) 

section .text
bits 32
default rel
global check_cpu

check_cpu:
    pushfd
    pop eax

    mov ecx, eax

    xor eax, 0x200000

    push eax
    popfd

    pushfd
    pop eax

    push ecx
    popfd

    ; Compare EAX and ECX. If they are equal then that means the bit
    ; wasn't flipped, and CPUID isn't supported.
    cmp eax, ecx
    je .unsupport_cpu

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

    ret

.unsupport_cpu:
    ; Handling Unsupported CPU
    mov dword [0xb8000], 0x0f6e0f75  ; "un"
    mov dword [0xb8004], 0x0f700f73  ; "sp"
    mov dword [0xb8008], 0x0f6f0f70  ; "po"
    mov dword [0xb800c], 0x0f740f72  ; "rt"
    mov dword [0xb8010], 0x0f200f20  ; "  "
    mov dword [0xb8014], 0x0f750f63  ; "cp"
    mov dword [0xb8018], 0x0f000f75  ; "u" + 终止
    hlt

