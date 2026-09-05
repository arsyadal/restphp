use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=c/sapi.c");
    println!("cargo:rerun-if-changed=c/sapi.h");

    // Discover linker search paths via php-config --ldflags
    if let Ok(ldflags_output) = Command::new("php-config").arg("--ldflags").output() {
        let ldflags_str = String::from_utf8_lossy(&ldflags_output.stdout);
        for flag in ldflags_str.split_whitespace() {
            if let Some(path) = flag.strip_prefix("-L") {
                println!("cargo:rustc-link-search=native={}", path);
            }
        }
    }
    println!("cargo:rustc-link-search=native=/usr/lib");
    println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
    println!("cargo:rustc-link-search=native=/usr/lib/php/20240924");
    println!("cargo:rustc-link-lib=php");

    // Discover include paths via php-config --includes
    let output = Command::new("php-config")
        .arg("--includes")
        .output()
        .expect("Failed to execute php-config. Ensure php-dev or libphp-embed is installed.");

    let includes_str = String::from_utf8_lossy(&output.stdout);
    let mut build = cc::Build::new();
    build.file("c/sapi.c");

    for flag in includes_str.split_whitespace() {
        if let Some(path) = flag.strip_prefix("-I") {
            build.include(path);
        }
    }

    build.include("c");
    build.define("_GNU_SOURCE", None);
    build.opt_level(3);
    build.compile("restphp_sapi");
}
