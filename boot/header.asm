; Define a section "multiboot2"
section .multiboot2
align 8	   ; The alignment required
begin:
    dd 0xE85250D6	; Magic number
    dd 0 		; Architecture, 0 is protected mode
    dd end - begin	; Header length
    dd 0x100000000 - (0xE85250D6 + 0 + (end - begin))	; Checksum

align 8		; The alignment required
framebuffer_tag:
    ; Framebuffer tag
    dw 5        ; Tag type, 5 is framebuffer
    dw 0	; Flags, needed
    dd 20	; Tag size
    dd 1024	; The width
    dd 768	; The height
    dd 32	; The depth

align 8		; The alignment required
end_tag:
    ; End tag
    dw 0	; Tag type, 0 is end
    dw 0	; Flags
    dd 8	; Tag size

end:
