# Proka Kernel - A kernel for ProkaOS
# Copyright (C) RainSTR Studio 2025, All Rights Reserved.
#
# This is the Makefile, which will build the C and ASM code, in
# order to help us to make a Rust kernel more easily.
#
#
.PHONY: mkdir clean debug
# Define some basic variables
BUILD_DIRS = boot tests 
OBJ_DIR = $(PWD)/target/obj
LDFLAGS = -nostdlib

# Build the clean codes (easy, just run the Makefile in each dirs)
all: clean mkdir
        # Iterate all the BUILD_DIRS.
	$(foreach dir, $(BUILD_DIRS), make -C $(dir) OBJ_DIR=$(OBJ_DIR);)

# Build the debug codes
debug: clean mkdir
	$(foreach dir, $(BUILD_DIRS), make -C $(dir) OBJ_DIR=$(OBJ_DIR) DEBUG=1;)

mkdir:
	mkdir -p $(OBJ_DIR)

clean:
	rm -rf $(OBJ_DIR)
