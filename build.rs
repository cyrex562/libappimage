use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Build the Rust library
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=include/");
    println!("cargo:rerun-if-changed=examples/");

    // Generate C bindings
    let bindings = bindgen::Builder::default()
        .header("include/appimage.h")
        .generate_comments(true)
        .clang_arg("-I/usr/include")
        .clang_arg("-I/usr/lib/gcc/x86_64-linux-gnu/14/include")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Copy header file to output directory
    std::fs::copy("include/appimage.h", out_path.join("appimage.h"))
        .expect("Failed to copy header file");

    // Build C example
    let cc = cc::Build::new();
    let compiler = cc.get_compiler();

    Command::new(compiler.path())
        .args(&[
            "-o",
            &out_path.join("appimage_example").to_str().unwrap(),
            "examples/appimage_example.c",
            "-I",
            "include",
            "-L",
            &out_path.to_str().unwrap(),
            "-lappimage",
            "-Wl,-rpath,$ORIGIN",
        ])
        .status()
        .expect("Failed to build C example");

    // Link against system libraries
    println!("cargo:rustc-link-lib=dylib=squashfuse");
    println!("cargo:rustc-link-lib=dylib=archive");
    println!("cargo:rustc-link-lib=dylib=elf");
}
