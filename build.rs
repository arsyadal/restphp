fn main() {
    println!("cargo:rustc-link-lib=php");
    println!("cargo:rerun-if-changed=build.rs");
}
