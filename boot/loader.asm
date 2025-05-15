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
default rel
bits 32
global _start

_start:
    mov [mb_magic], eax	; Save EAX value
    mov [mbi_ptr], ebx 	; Save EBX value
    mov esp, stack_top	; Set up the stack pointer
    
    call check_cpu ; Check for CPUID support
    
    ; 设置页表
    call setup_page_tables

    ; 启用PAE
    mov eax, cr4
    or eax, (1 << 5)     ; CR4.PAE = 1
    mov cr4, eax

    ; 设置EFER.LME
    mov ecx, 0xC0000080  ; EFER MSR编号
    rdmsr
    or eax, (1 << 8)     ; 设置LME位
    wrmsr

    ; 启用分页
    mov eax, cr0
    or eax, (1 << 31)    ; CR0.PG = 1
    mov cr0, eax

    ; 加载64位GDT并跳转
    lgdt [gdt64_ptr]
    jmp 0x08:long_mode_entry
    
bits 64
extern kernel_main
long_mode_entry:
    ; 更新段寄存器
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; 调用64位内核主函数
    mov edi, [mb_magic]  ; 传递multiboot参数
    mov esi, [mbi_ptr]
    call kernel_main
    hlt
    
bits 32
setup_page_tables:
    ; 初始化PML4
    mov eax, pdp_table
    or eax, 0x3          ; Present + Writable
    mov [pml4_table], eax

    ; 初始化PDP
    mov eax, pd_table
    or eax, 0x3
    mov [pdp_table], eax

    ; 初始化PD为1GB大页
    mov ecx, 0
    mov eax, 0x83        ; Present + Writable + LargePage
    
.set_pd_entries:
    mov [pd_table + ecx * 8], eax
    add eax, 0x40000000  ; 每个条目映射1GB
    inc ecx
    cmp ecx, 4           ; 映射前4GB内存
    jl .set_pd_entries

    ; 设置CR3指向PML4
    mov eax, pml4_table
    mov cr3, eax
    ret
    
check_cpu:
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
    
    ret

.unsupport_cpu:
    ; Handling Unsupported CPU
    hlt
    jmp .unsupport_cpu

section .bss
align 4096
pml4_table:
    resb 4096
pdp_table:
    resb 4096
pd_table:
    resb 4096

section .rodata
gdt64:
    dq 0                         ; 空描述符
    dq 0x0020980000000000        ; 代码段描述符 (执行/读, 64位)
    dq 0x0000920000000000        ; 数据段描述符 (读/写)
gdt64_ptr:
    dw $ - gdt64 - 1             ; GDT长度-1
    dq gdt64                     ; GDT基地址
