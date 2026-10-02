// Derived by fid-hardware from hardware/product.toml — do not edit.
// A renamed net or a moved pin changes this file in the same `fid derive`.

export const NETS = ["1V1","3V3","CC1","CC2","GND","LED_DIN","QSPI_SCLK","QSPI_SD0","QSPI_SD1","QSPI_SD2","QSPI_SD3","QSPI_SS","SENSOR_SCL","SENSOR_SDA","USB_DM","USB_DM_MCU","USB_DP","USB_DP_MCU","VBUS","XIN","XOUT","XOUT_X"] as const;
export type Net = (typeof NETS)[number];

/** The MCU pin each net is on, for nets the firmware drives. */
export const MCU_PINS = {"LED_DIN":"GPIO16","SENSOR_SCL":"GPIO5","SENSOR_SDA":"GPIO4"} as const;

/** Sockets reached through the case: the part, the wall, the window. */
export const CONNECTORS = [{"opening_mm":[12.5,6.5],"part":"usb","side":"bottom"}] as const;

export const BOARD_MM = {"size":[26.0,79.0],"thickness":1.6} as const;
