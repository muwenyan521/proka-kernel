# Proka Kernel - A kernel for ProkaOS
# Copyright (C) RainSTR Studio 2025, All Rights Reserved.
#
# This is the Makefile, which will build the Assembly code
# only and link with C code.

# The ASM source
ASM_CODE := $(wildcard *.asm)

# The object file
OBJ_FILES := $(patsubst %.asm,$(OBJ_DIR)/%.o,$(ASM_CODE))

# Build
all: $(OBJ_FILES)

$(OBJ_DIR)/%.o: %.asm
	nasm -f elf64 $< -o $@ -g
