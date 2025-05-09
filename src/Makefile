# Proka Kernel - A kernel for ProkaOS
# Copyright (C) RainSTR Studio 2025, All Rights Reserved.
#
# This is the Makefile, which will build the C and ASM code, in
# order to help us to make a Rust kernel more easily.
#
#

.PHONY: asm c
# Define some basic variables (ONLY ALLOW TO USE IN PROJECT ROOT)
ASM_SRC = asm
C_SRC = c
OBJ_DIR = $(PWD)/target/obj
TARGET = kernel
LDFLAGS = -nostdlib

# Build the codes (easy, just run the Makefile in each dirs)
all: mkdir asm c

asm:
	make -C $(ASM_SRC) OBJ_DIR=$(OBJ_DIR)

c:
	make -C $(C_SRC) OBJ_DIR=$(OBJ_DIR)

mkdir:
	mkdir -p $(OBJ_DIR)

