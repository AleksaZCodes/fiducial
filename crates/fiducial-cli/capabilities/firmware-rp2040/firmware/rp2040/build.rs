fn main() {
    // memory.x sits beside this file; cargo runs the linker from the
    // workspace root, so "." would not find it.
    println!("cargo:rustc-link-search={}", env!("CARGO_MANIFEST_DIR"));
    println!("cargo:rerun-if-changed=memory.x");
}
