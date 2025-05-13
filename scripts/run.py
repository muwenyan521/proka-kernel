#!/bin/env python

import os
import subprocess
import argparse
import shutil
import sys


def run_command(cmd, with_root=False):
    program = cmd.split()[0]
    full_cmd = f"sudo {cmd}" if with_root else cmd
    try:
        subprocess.run(full_cmd, check=True, shell=True, env=os.environ)
    except subprocess.CalledProcessError as e:
        print(f'Error running "{program}": {e}')
        sys.exit(1)
    except FileNotFoundError:
        print(f'Command not found: "{program}"')
        sys.exit(1)


def get_available_loop_device():
    try:
        output = subprocess.check_output(
            "sudo losetup -f", shell=True, stderr=subprocess.STDOUT, text=True
        )
        return output.strip()
    except subprocess.CalledProcessError as e:
        print(f"Error finding loop device: {e}")
        return None


def make_disk(image_file, size_mb, mount_point):
    loop_device = None
    mounted = False

    try:
        # Create disk image
        print(f"Creating {size_mb}MB disk image...")
        run_command(f"dd if=/dev/zero of={image_file} bs=1M count={size_mb}")

        # Partition disk
        print("Partitioning disk...")
        run_command(
            f"parted -s {image_file} mklabel msdos mkpart primary ext4 1MiB 100%"
        )

        # Setup loop device
        loop_device = get_available_loop_device()
        if not loop_device:
            raise RuntimeError("No available loop devices")

        print(f"Using loop device: {loop_device}")
        run_command(f"losetup -P {loop_device} {image_file}", True)

        # Format partition
        print("Formatting partition...")
        run_command(f"mkfs.ext4 {loop_device}p1", True)

        # Mount partition
        print(f"Mounting to {mount_point}...")
        os.makedirs(mount_point, exist_ok=True)
        run_command(f"mount {loop_device}p1 {mount_point}", True)
        mounted = True

        # Install GRUB
        boot_dir = os.path.join(mount_point, "boot")
        print("Installing GRUB...")
        run_command(
            f"grub-install --target=i386-pc --boot-directory={boot_dir} {loop_device}",
            True,
        )

        # Create GRUB config
        grub_dir = os.path.join(boot_dir, "grub", 'grub.cfg')
        os.makedirs(os.path.dirname(grub_dir), exist_ok=True)

        grub_cfg = """\
set root=hd0,1
menuentry "Proka OS" {
    multiboot2 /boot/proka-kernel
    boot
}"""

        subprocess.run(f'echo "{grub_cfg}" | sudo tee {grub_dir} > {os.path.devnull}', check=True, shell=True)
        
        # Copy kernel
        kernel_src = "target/x86_64-unknown-none/debug/proka-kernel"
        if not os.path.exists(kernel_src):
            raise FileNotFoundError(f"Kernel not found at {kernel_src}")

        print("Copying kernel...")
        dst = os.path.join(boot_dir, "proka-kernel")
        run_command(f'cp {kernel_src} {dst}', True)
        
    finally:
        # Cleanup
        if mounted:
            print("Unmounting...")
            run_command(f"umount {mount_point}", True)
        if loop_device:
            print("Releasing loop device...")
            run_command(f"losetup -d {loop_device}", True)

def update_disk(image_file, mount_point):
    loop_device = None
    mounted = False

    try:

        # Setup loop device
        loop_device = get_available_loop_device()
        if not loop_device:
            raise RuntimeError("No available loop devices")

        print(f"Using loop device: {loop_device}")
        run_command(f"losetup -P {loop_device} {image_file}", True)

        # Mount partition
        print(f"Mounting to {mount_point}...")
        os.makedirs(mount_point, exist_ok=True)
        run_command(f"mount {loop_device}p1 {mount_point}", True)
        mounted = True
        boot_dir = os.path.join(mount_point, "boot")
        
        # Copy kernel
        kernel_src = "target/x86_64-unknown-none/debug/proka-kernel"
        if not os.path.exists(kernel_src):
            raise FileNotFoundError(f"Kernel not found at {kernel_src}")

        print("Copying kernel...")
        dst = os.path.join(boot_dir, "proka-kernel")
        run_command(f'cp {kernel_src} {dst}', True)
        
    finally:
        # Cleanup
        if mounted:
            print("Unmounting...")
            run_command(f"umount {mount_point}", True)
        if loop_device:
            print("Releasing loop device...")
            run_command(f"losetup -d {loop_device}", True)


def build_kernel():
    run_command("env cargo build")


def run_qemu(image_file, param):
    qemu_cmd = f"qemu-system-x86_64 -drive format=raw,file={image_file}"
    if param:
        qemu_cmd += f' {" ".join(param)}'
    print("Qemu parameter:", qemu_cmd)
    try:
        run_command(qemu_cmd)
    except KeyboardInterrupt:
        pass
    

def main():
    parser = argparse.ArgumentParser(description="Create bootable Proka OS disk image")
    parser.add_argument(
        "-f", "--image-file", default="proka.img", help="Output image filename"
    )
    parser.add_argument("-s", "--size", type=int, default=64, help="Image size in MB")
    parser.add_argument(
        "-m", "--mount-dir", default="mnt", help="Temporary mount directory"
    )
    parser.add_argument(
        "-p", "--qemu-param", nargs="*", default=[], help="Additional QEMU parameters"
    )
    parser.add_argument('--force', action='store_true', default=False, help='Force image file to be recreated')
    args = parser.parse_args()

    build_kernel()
    print("\nKernel compiled successfully!")
    if not args.force and os.path.exists(args.image_file):
        print('Use an existing image')
        update_disk(args.image_file, args.mount_dir)
    else:
        make_disk(args.image_file, args.size, args.mount_dir)
    print("\nDisk image created successfully!")
    print(f" -> Image file: {args.image_file}")
    run_qemu(args.image_file, args.qemu_param)


if __name__ == "__main__":
    main()
