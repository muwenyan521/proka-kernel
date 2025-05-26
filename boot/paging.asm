section .bss
align 4096
p4_table:
    resb 4096
p3_table:
    resb 4096
p2_table:
    resb 4096
p1_tables:
    resb 512 * 4096	; 512 P1 tables

section .text
bits 32
global set_up_page_tables
global enable_paging
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

    ; map each P2 entry to a 2MiB page
    mov ecx, 0         ; counter variable

.map_p2_table:
    ; Compute the current P1: p1_tables + ecx * 4096
    mov eax, p1_tables          ; P1 table array base addr
    mov ebx, ecx
    shl ebx, 12                 ; ebx = ecx * 4096 (each P1 table needs 4KB)
    add eax, ebx                ; eax = The physical addr in P1
    or eax, 0x3                 ; Present + Writable
    mov [p2_table + ecx*8], eax ; Write to the ecx-th of P2

    ; Fill the 512 indexes of the current P1 (map 4KiB page)
    mov edi, eax                ; edi point at the current P1
    and edi, 0xFFFFF000         ; Clean the low addr and stay the physical address
    mov ebx, 0                  ; P1 table index (0~511)

.map_p1_table:
    ; Compute physical addr: (ecx * 2MB) + (ebx * 4KB)
    mov eax, ecx
    shl eax, 21                 ; eax = ecx * 2MB (Every P1 table manages 2MiB space)
    mov edx, ebx
    shl edx, 12                 ; edx = ebx * 4KB
    add eax, edx                ; eax = Final physical addr
    or eax, 0x3                 ; Present + Writable
    mov [edi + ebx*8], eax      ; Write to the ebx-th of P1

    inc ebx
    cmp ebx, 512
    jl .map_p1_table

    inc ecx
    cmp ecx, 512
    jl .map_p2_table

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
