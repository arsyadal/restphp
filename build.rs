use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=c/sapi.c");
    println!("cargo:rerun-if-changed=c/sapi.h");
    println!("cargo:rerun-if-env-changed=PHP_CONFIG");
    println!("cargo:rerun-if-env-changed=PHP_EMBED_LIB_NAME");
    println!("cargo:rerun-if-env-changed=PHP_EMBED_LIB_DIR");

    let php_config = std::env::var_os("PHP_CONFIG").unwrap_or_else(|| "php-config".into());
    let config = |option: &str| {
        let output = Command::new(&php_config)
            .arg(option)
            .output()
            .expect("Failed to execute PHP_CONFIG (default: php-config). Install matching PHP development headers.");
        assert!(output.status.success(), "PHP_CONFIG {option} failed");
        String::from_utf8(output.stdout).expect("PHP_CONFIG output must be UTF-8")
    };
    for flag in config("--ldflags").split_whitespace() {
        if let Some(path) = flag.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={path}");
        }
    }
    if let Some(directory) = std::env::var_os("PHP_EMBED_LIB_DIR") {
        println!(
            "cargo:rustc-link-search=native={}",
            std::path::PathBuf::from(directory).display()
        );
    }
    let library = std::env::var("PHP_EMBED_LIB_NAME").unwrap_or_else(|_| "php".into());
    println!("cargo:rustc-link-lib={library}");

    let includes = config("--includes");

    let includes_str = includes.as_str();
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
