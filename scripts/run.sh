#!/bin/bash
# Proka Kernel - A kernel for ProkaOS
# Copyright (C) RainSTR Studio 2025, All Rights Reserved.
# This script helps run the kernel using GRUB.

# Function to create disk image and related operations
make_disk() {
    # Create a 64MB empty image file
    dd if=/dev/zero of=disk.img bs=1M count=64

    # Create MS-DOS partition table and primary partition with ext2 file system
    parted -s disk.img mklabel msdos mkpart primary ext2 1MiB 100%

    # Map image file to loop device /dev/loop0 and detect partition table
    sudo losetup -P /dev/loop0 disk.img

    # Create ext2 file system on partition /dev/loop0p1
    sudo mkfs.ext2 /dev/loop0p1

    # Mount partition /dev/loop0p1 to /mnt directory
    sudo mount /dev/loop0p1 /mnt

    # Install GRUB bootloader on /dev/loop0, target architecture i386-pc, boot directory /mnt/boot
    sudo grub-install --target=i386-pc --boot-directory=/mnt/boot /dev/loop0

    # Create /mnt/boot/grub directory, -p creates parent dirs if not exist
    sudo mkdir -p /mnt/boot/grub

    # Write the grub config file
    echo 'set root=hd0,1
menuentry "Proka Kernel" {
    multiboot2 /boot/proka-kernel
    boot
}' | sudo tee /mnt/boot/grub/grub.cfg > /dev/null

    # Move kernel file to /mnt/boot directory
    sudo cp target/x86_64-unknown-none/debug/proka-kernel /mnt/boot

    # Unmount the partition
    sudo umount /mnt

    # Unmap the loop device
    sudo losetup -d /dev/loop0
}
# Build the kernel first
cargo build

# Check arguments
if [ $# -ne 0 ]; then
    qemu_arguments="$@"
else
    qemu_arguments=""
fi

# Check if disk.img exists, if not, call make_disk function
if [ ! -f disk.img ]; then
    echo "NOTE: The disk image is not exist, making..."
    make_disk
fi

# Run QEMU with disk image and arguments
qemu-system-x86_64 -hda disk.img $qemu_arguments
