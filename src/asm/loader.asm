; Proka Kernel - A kernel for ProkaOS.
; Copyright (C) RainSTR Studio 2025, All Rights Reserved.
;
; This file contains the header of multiboot2, which can
; boot up by using GRUB.

; Define a section "multiboot2"
section .multiboot2
align 4	   ; The alignment required
begin:
    dd 0xE85250D6	; Magic number
    dd 0 		; Architecture, 0 is protected mode
    dd end - begin	; Header length
    dd 0x100000000 - (0xE85250D6 + 0 + (end - begin))	; Checksum
    
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

; The entry of the program
section .text
extern kernel_main
global _start

_start:
    mov esp, stack_top	; Set up the stack pointer
    ; TODO: Write the cide that switch to the long mode.
    
    ; Just enter the main kernel function
    call kernel_main
    
    ; Usually, the code shouldn't run these codes
    hlt
