// Proka Kernel - A kernel for ProkaOS
// Copyright (C) RainSTR Studio 2025, All Rights Reserved.
//
// The build script, which can link the C and ASM code.

// Import some modules
use glob::glob;
use std::path::Path;
use std::process::Command; // For checking the .o file

fn main() {
    // Get the workspace root
    let workspace_root = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .expect("CARGO_MANIFEST_DIR should be doubly nested in workspace")
        .to_path_buf();

    let status = Command::new("make")
        .arg("-C")
        .arg("..") // Because the main Makefile is at ..
        .status()
        .expect("Cannot run command");

    if !status.success() {
        panic!("Building C/ASM NOT successful");
    }

    let _ = Command::new("pwd");

    // Tell Rust to link the ELF file generated
    println!("cargo:rustc-link-arg=-Tlinker.ld");
    println!("cargo:rustc-link-arg=-nostdlib");
    println!("cargo:rustc-link-arg=-no-pie");

    // Check the file should link
    let obj_dir = workspace_root.join("target/obj");

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
