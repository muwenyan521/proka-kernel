// Proka Kernel - A kernel for ProkaOS
// Copyright (C) RainSTR Studio 2025, All Rights Reserved.
//
// The build script, which can link the C and ASM code.

// Import some modules
use glob::glob;
use std::path::Path;

fn main() {
    // Tell Rust to link the ELF file generated
    println!("cargo:rustc-link-arg=-Tlinker.ld");
    println!("cargo:rustc-link-arg=-nostdlib");
    println!("cargo:rustc-link-arg=-no-pie");

    // Check the file should link
    let obj_dir = Path::new("target/obj");

    if let Ok(paths) = glob(&format!("{}/*.o", obj_dir.display())) {
        for path_result in paths {
            if let Ok(path) = path_result {
                // Get the absolute path
                let absolute_path = path.canonicalize().expect("Failed to canonicalize path");
                println!("cargo:rustc-link-arg={}", absolute_path.display());
            }
        }
    }
}
