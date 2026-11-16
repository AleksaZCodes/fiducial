# {{name}} — ESP32-C6 Firmware

esp-hal and Embassy firmware for the ESP32-C6, part of the `{{name}}` product.

```sh
cd firmware/esp32c6
cargo build --release     # compile and link
cargo run --release       # flash and open the serial monitor (needs espflash)
```

The toolchain pin is `firmware/rust-toolchain.toml`. See `.agents/skills/fiducial-firmware-esp32c6/SKILL.md`
for how the pieces fit and why their versions move together.
