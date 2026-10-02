# Vendored components

**From LCSC's library**, fetched with `python3 hardware/parts.py fetch <LCSC>`
(easyeda2kicad 1.0.1, converted to the current KiCad format): symbol,
footprint and 3D model, made for one another, where KiCad has no entry (the
sensor) or no 3D model (the flash).

| Library entry | LCSC | Part |
|---|---|---|
| `lcsc:AHT20_C2757850`, `lcsc:SENSOR-SMD_L3.0-W3.0-P1.00-BR` | C2757850 | Aosong AHT20, I²C temperature and humidity sensor, DFN-6 3 × 3 mm |
| `lcsc:W25Q32JVSSIQ_C179173`, `lcsc:SOIC-8_L5.3-W5.3-P1.27-LS8.0-BL` | C179173 | Winbond W25Q32JVSSIQ, 32 Mbit QSPI flash, SOIC-8 208 mil |

**KiCad's land pattern with LCSC's model.** `Connector_USB.pretty/USB_C_Receptacle_HRO_TYPE-C-31-M-12`
is KiCad 7.0.11's footprint for the Hroparts TYPE-C-31-M-12 (C165948), unchanged
but for its `model` line. KiCad's 3D library has no model for it. LCSC's
footprint for the same part merges pads to 0.10 mm apart and draws its
locating pegs as plated holes with no ring, so KiCad's DRC rejects it.
Its STEP model (`Connector_USB.3dshapes/`, from `parts.py fetch C165948`)
is placed on KiCad's pattern: turned 180° as LCSC turns it, and moved
1.05 mm toward the mouth (−y in model coordinates). That offset was measured
on the model itself: it centres the model's four shell legs and two locating
pegs on KiCad's six drilled holes, all within 0.01 mm. The two footprints'
own drawings differ by 1.41 mm, so LCSC's model does not sit exactly on
LCSC's footprint either.
