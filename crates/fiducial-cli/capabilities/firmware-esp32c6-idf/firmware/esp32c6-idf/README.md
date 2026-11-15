# {{name}} — ESP32-C6 firmware on ESP-IDF

std Rust on the ESP-IDF SDK (FreeRTOS, the Wi-Fi and MQTT stacks and the rest of
the SDK), part of the `{{name}}` product. The bare-metal Embassy alternative is
`fid add firmware esp32c6`.

## Build and flash

```sh
cargo install ldproxy espflash    # once
cd firmware/esp32c6-idf
cargo build --release             # the first build downloads and builds the SDK (large, slow)
cargo run --release               # flash and open the serial monitor
```

Nightly Rust and `build-std` are pinned in `rust-toolchain.toml` and
`.cargo/config.toml`; the ESP-IDF version is `ESP_IDF_VERSION` in the latter.
