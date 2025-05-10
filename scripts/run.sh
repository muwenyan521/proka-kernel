#!/bin/bash
# Proka Kernel - A kernel for ProkaOS
# Copyright (C) RainSTR Studio 2025, All Rights Reserved.
#
# This file can help you to run the kernel by using grub.

# Check the arguments
if [ $# -ne 0 ]; then
    # Append these arguments to QEMU
    qemu_arguments="$@"
else
    qemu_arguments=""
fi

# First, build it with Cargo.
cargo build 

# Create a 64MB empty image file
dd if=/dev/zero of=disk.img bs=1M count=64

# Use parted to create an MS-DOS partition table and a primary partition with ext2 file system
# -s option means silent mode, no interaction
parted -s disk.img mklabel msdos mkpart primary ext2 1MiB 100%

# Map the image file to the loop device /dev/loop0 and auto-detect the partition table
sudo losetup -P /dev/loop0 disk.img

# Create an ext2 file system on the partition /dev/loop0p1
sudo mkfs.ext2 /dev/loop0p1

# Mount the partition /dev/loop0p1 to the /mnt directory
sudo mount /dev/loop0p1 /mnt

# Install the GRUB bootloader on the /dev/loop0 device, target architecture is i386-pc, boot directory is /mnt/boot
sudo grub-install --target=i386-pc --boot-directory=/mnt/boot /dev/loop0

# Create the /mnt/boot/grub directory, -p option creates parent directories if they don't exist
sudo mkdir -p /mnt/boot/grub

# Move the kernel file to the /mnt/boot directory
sudo mv target/x86_64-unknown-none/debug/proka-kernel /mnt/boot

# Unmount the partition
sudo umount /mnt

# Unmap the loop device
sudo losetup -d /dev/loop0

# Run QEMU with the disk image
qemu-system-x86_64 -hda disk.img $qemu_arguments

