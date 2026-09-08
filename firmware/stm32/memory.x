/* STM32F401CC memory map — 256 KB flash, 64 KB RAM.
 * Used by the STM32F401 "blackpill" v2 board.
 * Adjust ORIGIN/LENGTH for your variant:
 *   STM32F401CB: 128K flash | STM32F401CD: 384K | STM32F401CE: 512K, 96K RAM
 */
MEMORY {
    FLASH : ORIGIN = 0x08000000, LENGTH = 256K
    RAM   : ORIGIN = 0x20000000, LENGTH = 64K
}
