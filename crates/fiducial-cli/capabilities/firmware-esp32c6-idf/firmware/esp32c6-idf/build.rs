fn main() {
    // Exports the ESP-IDF build's link arguments and cfgs to this crate.
    embuild::espidf::sysenv::output();
}
