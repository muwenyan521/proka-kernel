# Proka Kernel - A kernel for ProkaOS
Copyright (C) RainSTR Studio 2025, All rights reserved.

Welcome to use Proka Kernel, invented by RainSTR Studio, which is made up by young developers.
 
This project is for practise only, but we hope that it will use in stable environment.

## What language does it written?
Well, if you have seen the *languages* part, you'll find **C, Assembly and Rust**. Yes, this is the language what does the Proka Kernel written.

You know, **Rust** is the memory-safe language, so the kernel mainly written in Rust. Also, for some low-level operation and drivers, we uses **C and Assembly** to do that.

Also, you will find that it also have **Python**. But why? Can Python write kernels? Of course no, but as a interpreted language, it is a script that can run, debug or do something else easily.

## How to build?
### Requirements
Well, If you want to build this project, you need to install these components:
- Rust (nightly, with target `x86_64-unknown-none`, the Rust compiler);
- GCC (The C code compiler);
- NASM (The Assembly code compiler);
- Make (The build tools of C and NASM codes).

If you want to run it in your operating system, you also need to install these components:
- QEMU (The kernel emulater);
- losetup* (A command, which will mount to loop device);
- dd* (A command, which will make up a disk image);
- parted* (A part tool);
- GRUB (The bootloader);
- Python (The script runner).

NOTE: The components with `*` means that it may pre-installed in your operating system.

### Install commands
We suggest you use `rustup` to install Rust.

If you're using Debian or Ubuntu, you can try:
```bash
sudo apt-get install gcc nasm make     # Must install
sudo apt-get install qemu-system-x86 parted grub-efi-amd64 python3    # If you want to run the kernel in your OS
```

### Build
To build it, you just need to run 1 command:
```bash
make
```

The kernel file will put in `kernel/kernel` in project root.

Isn't it easy? If you want to make up an ISO, you just need to run:
```bash
make makeiso
```

The ISO file will put in `proka-kernel.iso` in project root.

If you want to start the emulation of the kernel by using QEMU, you just need to run:
```bash
make run  # Must run in project root!!!
```

Then the QEMU process will on. You can see the kernel log in the terminal.

## Contributors
- zhangxuan2011 <zx20110412@outlook.com>
- moyan <moyan@moyanjdc.top>
- xiaokuai <rainyhowcool@outlook.com>
- TMX <273761857@qq.com>

