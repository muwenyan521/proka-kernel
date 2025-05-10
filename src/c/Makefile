# Proka Kernel - A kernel for ProkaOS
# Copyright (C) RainSTR Studio 2025, All Rights Reserved.
#
# This is the Makefile, which will build the C code
# only and link with C code.

# The ASM source
C_CODE = $(wildcard *.c)
CFLAGS = -std=gnu99 -ffreestanding -O0 -Wall -Wextra -fPIE -fno-stack-protector

# The object file
OBJ_FILES = $(patsubst %.c,$(OBJ_DIR)/%.o,$(C_CODE))

# Build
all: $(OBJ_FILES)

$(OBJ_DIR)/%.o: %.c
	gcc -c $< -o $@
