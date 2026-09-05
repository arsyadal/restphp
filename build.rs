use std::process::Command;

fn main() {
    println!("cargo:rustc-link-lib=php");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=c_src/sapi_bridge.c");
    println!("cargo:rerun-if-changed=c_src/sapi_bridge.h");

    // Discover include paths via php-config
    let output = Command::new("php-config")
        .arg("--includes")
        .output()
        .expect("Failed to execute php-config");

    let includes_str = String::from_utf8_lossy(&output.stdout);
    let mut build = cc::Build::new();
    build.file("c_src/sapi_bridge.c");

    for flag in includes_str.split_whitespace() {
        if let Some(path) = flag.strip_prefix("-I") {
            build.include(path);
        }
    }

    build.include("c_src");
    build.opt_level(3);
    build.compile("restphp_sapi_bridge");
}
