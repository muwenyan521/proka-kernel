// Proka Kernel - A kernel for ProkaOS
// Copyright (C) RainSTR Studio 2025, All Rights Reserved.
//
// The build script, which can link the C and ASM code.

// Import some modules
use glob::glob;
use std::process::Command; // For checking the .o file

fn main() {
    // First, run "make" to build the ASM and C code
    let status = Command::new("make")
        .arg("-C")
        .arg("src")
        .status()
        .expect("Cannot run command");

    if !status.success() {
        panic!("Building C/ASM NOT successful");
    }

    // Tell Rust to link the ELF file generated
    println!("cargo:rustc-link-arg=-Tlinker.ld");
    println!("cargo:rustc-link-arg=-nostdlib");
    println!("cargo:rustc-link-arg=-no-pie");

    // Check the file should link
    if let Ok(paths) = glob("target/obj/*.o") {
        for path in paths {
            if let Ok(path) = path {
                println!("cargo:rustc-link-arg={}", path.display());
            }
        }
    }
}
