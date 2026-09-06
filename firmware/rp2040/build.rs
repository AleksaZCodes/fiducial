/// Build script — tells cargo to re-link if the linker script changes,
/// and adds the firmware directory to the linker search path so `cortex-m-rt`
/// can find `memory.x`.
fn main() {
    println!("cargo:rustc-link-search=.");
    println!("cargo:rerun-if-changed=memory.x");
}
