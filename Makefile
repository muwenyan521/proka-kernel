# Proka Kernel - A kernel for ProkaOS
# Copyright (C) RainSTR Studio 2025, All Rights Reserved.
#
# This is the Makefile, which will build the C and ASM code, in
# order to help us to make a Rust kernel more easily.
#
#
.PHONY: clean debug run iso
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
all:
    # Iterate all the BUILD_DIRS.
	$(foreach dir, $(BUILD_DIRS), make -C $(dir) OBJ_DIR=$(OBJ_DIR);)

## Build the ISO image
# This code is from TMXQWQ/TKernel2 in github
iso: all kernel/kernel initrd
	mkdir -p iso
	cp -r ./assets/rootfs/* ./iso/
	cp ./assets/initrd.cpio ./iso/initrd.cpio
	rm -f ./assets/initrd.cpio
	cp ./kernel/kernel ./iso/kernel
	touch ./proka-kernel.iso
	xorriso $(XORRISOFLAGS) ./iso -o ./proka-kernel.iso \
	  2> /dev/null
	rm -rf ./iso
	@echo "ISO image built: proka-kernel.iso"

initrd:
	cd ./assets/initrd && find . -print | cpio -H newc -v -o > ../initrd.cpio && cd ../..

run: iso
	qemu-system-x86_64 -enable-kvm $(QEMU_FLAGS) $(QEMU_OUT)
	@echo "QEMU started"

debug: iso
	qemu-system-x86_64 -enable-kvm $(QEMU_FLAGS) $(QEMU_OUT) -s -S

clean:
	make -C kernel clean
	rm -rf proka-kernel.iso

