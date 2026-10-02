# Vendored components

Fetched with `python3 hardware/parts.py fetch <LCSC>` (easyeda2kicad 1.0.1)
from EasyEDA's library for the LCSC part named. No 3D model was reachable for
either; `cad.py` builds their bodies from the footprints.

| Library entry | LCSC | Part |
|---|---|---|
| `lcsc:AHT20_C2757850`, `lcsc:SENSOR-SMD_L3.0-W3.0-P1.00-BR` | C2757850 | Aosong AHT20, I²C temperature and humidity sensor, DFN-6 3 × 3 mm |
| `lcsc:AMS1117-3.3`, `lcsc:SOT-223-3_L6.5-W3.4-P2.30-LS7.0-BR` | C6186 | AMS1117-3.3, 1 A LDO, SOT-223 (KiCad's own symbol `extends` another, which `parts.py` does not flatten yet) |
