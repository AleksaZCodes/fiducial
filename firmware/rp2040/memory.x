/* RP2040 memory map — Raspberry Pi Pico (2 MB flash, 264 KB RAM).
 *
 * BOOT2 (256 B): the second-stage bootloader blob copied into SRAM at reset.
 *   embassy-rp bundles the correct blob; this region reserves space for it.
 * FLASH: XIP-accessible program flash.
 * RAM: on-chip SRAM (banks 0–3 + 4–5 striped).
 */
MEMORY {
    BOOT2 : ORIGIN = 0x10000000, LENGTH = 0x100
    FLASH : ORIGIN = 0x10000100, LENGTH = 2048K - 0x100
    RAM   : ORIGIN = 0x20000000, LENGTH = 256K
}
