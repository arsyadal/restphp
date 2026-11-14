use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=c/sapi.c");
    println!("cargo:rerun-if-changed=c/sapi.h");
    println!("cargo:rerun-if-env-changed=PHP_CONFIG");
    println!("cargo:rerun-if-env-changed=PHP_DEV_DIR");
    println!("cargo:rerun-if-env-changed=PHP_EMBED_LIB_NAME");
    println!("cargo:rerun-if-env-changed=PHP_EMBED_LIB_DIR");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let mut build = cc::Build::new();
    build.file("c/sapi.c");
    build.include("c");

    if target_os == "windows" {
        configure_windows(&mut build);
    } else {
        configure_php_config(&mut build);
    }

    if !build.get_compiler().is_like_msvc() {
        build.define("_GNU_SOURCE", None);
    }
    build.opt_level(3);
    build.compile("restphp_sapi");
}

fn configure_php_config(build: &mut cc::Build) {
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
            PathBuf::from(directory).display()
        );
    }
    let library = std::env::var("PHP_EMBED_LIB_NAME").unwrap_or_else(|_| "php".into());
    println!("cargo:rustc-link-lib={library}");

    let includes = config("--includes");
    for flag in includes.split_whitespace() {
        if let Some(path) = flag.strip_prefix("-I") {
            build.include(path);
        }
    }
}

fn configure_windows(build: &mut cc::Build) {
    assert_eq!(
        std::env::var("CARGO_CFG_TARGET_ENV").as_deref(),
        Ok("msvc"),
        "Windows requires the x86_64-pc-windows-msvc Rust target"
    );
    assert_eq!(
        std::env::var("CARGO_CFG_TARGET_ARCH").as_deref(),
        Ok("x86_64"),
        "Windows currently requires x64 PHP and Rust"
    );
    let dev_dir = PathBuf::from(std::env::var_os("PHP_DEV_DIR").expect(
        "Set PHP_DEV_DIR to the extracted x64 NTS PHP development pack (containing include/ and lib/)",
    ));
    let include_root = dev_dir.join("include");
    assert!(
        include_root.join("main/php.h").is_file(),
        "PHP_DEV_DIR must contain include/main/php.h"
    );
    for subdir in ["", "main", "Zend", "TSRM"] {
        build.include(include_root.join(subdir));
    }
    let lib_dir = std::env::var_os("PHP_EMBED_LIB_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| dev_dir.join("lib"));
    let library = std::env::var("PHP_EMBED_LIB_NAME").unwrap_or_else(|_| "php8".into());
    assert!(
        library == "php8" && lib_dir.join("php8.lib").is_file(),
        "Windows requires the matching NTS php8.lib; TS php8ts builds are not supported"
    );
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib={library}");
    build.define("PHP_WIN32", None);
    build.define("ZEND_WIN32", None);
    build.define("ZEND_DEBUG", "0");
}
