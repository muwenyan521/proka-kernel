# Proka Kernel - A kernel for ProkaOS
# Copyright (C) RainSTR Studio 2025, All Rights Reserved.
#
# This is the Makefile, which will build the C and ASM code, in
# order to help us to make a Rust kernel more easily.
#
#

# Define some basic variables
BUILD_DIRS = boot 
OBJ_DIR = $(PWD)/target/obj
LDFLAGS = -nostdlib

# Build the codes (easy, just run the Makefile in each dirs)
all: mkdir
	# Iterate all the BUILD_DIRS.
	$(foreach dir, $(BUILD_DIRS), make -C $(dir) OBJ_DIR=$(OBJ_DIR))


mkdir:
	mkdir -p $(OBJ_DIR)

