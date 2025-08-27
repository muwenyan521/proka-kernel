# Proka Kernel - A kernel for ProkaOS
# Copyright (C) RainSTR Studio 2025, All Rights Reserved.
#
# This is the Makefile, which will build the C and ASM code, in
# order to help us to make a Rust kernel more easily.
#
#
.PHONY: mkdir clean debug
# Define some basic variables
BUILD_DIRS = kernel
OBJ_DIR = $(PWD)/target/obj
LDFLAGS = -nostdlib
XORRISOFLAGS = -as mkisofs --efi-boot limine/limine-uefi-cd.bin
QEMU_FLAGS := -bios ./assets/OVMF.fd -cdrom proka-kernel.iso --machine q35 -m 1G
# QEMU_KVM := -enable-kvm -cpu host
# QEMU_reOUT := > ./qemu.log
QEMU_OUT := -serial stdio $(QEMU_reOUT)
# Build the clean codes (easy, just run the Makefile in each dirs)
all: clean mkdir
        # Iterate all the BUILD_DIRS.
	$(foreach dir, $(BUILD_DIRS), make -C $(dir) OBJ_DIR=$(OBJ_DIR);)

## Build the ISO image
# This code is from TMXQWQ/TKernel2 in github
makeiso: all kernel/kernel
	mkdir -p iso
	cp -r ./assets/rootfs/* ./iso/
	cp ./kernel/kernel ./iso/kernel
# 	cp ./initrd.img ./iso	# TODO: Support initrd
	touch ./proka-kernel.iso
	xorriso $(XORRISOFLAGS) ./iso -o ./proka-kernel.iso \
	  2> /dev/null
	rm -rf ./iso
	@echo "ISO image built: proka-kernel.iso"

run: makeiso
	qemu-system-x86_64 -enable-kvm $(QEMU_FLAGS) $(QEMU_OUT)
	@echo "QEMU started"

mkdir:
	mkdir -p $(OBJ_DIR)

clean:
	rm -rf $(OBJ_DIR)
