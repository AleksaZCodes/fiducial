#!/usr/bin/env bash
# hardware/product.toml → everything a manufacturer and a reviewer need.
#
#   fid derive --pipeline hardware       solve (gated): layout, BOM, board, schematic
#   python3 hardware/ratsnest.py         the placement and its ratsnest → build/review/ratsnest.svg
#   /usr/bin/python3 hardware/route.py   route, pour and DRC the board (KiCad's own Python)
#   python3 hardware/cad.py              solids + fit/assembly checks → hardware/build/
#   kicad-cli                            Gerbers, drill, STEP and the schematic PDF
#   node hardware/render.mjs             review pack → hardware/build/review/
#
# Platform-owned. Set SKIP_RENDER=1 where there is no browser (CI), and
# SKIP_ROUTE=1 to build the case without routing the board.
set -euo pipefail
cd "$(dirname "$0")/.."

fid derive --pipeline hardware
python3 hardware/ratsnest.py

pcb=hardware/generated/board.kicad_pcb
rm -f hardware/build/copper.json
if [ -f "$pcb" ] && [ "${SKIP_ROUTE:-0}" != "1" ]; then
  /usr/bin/python3 hardware/route.py
  pcb=hardware/build/board-routed.kicad_pcb
fi

python3 hardware/cad.py

if [ -f "$pcb" ]; then
  mkdir -p hardware/build/fab
  kicad-cli pcb export gerbers -o hardware/build/fab/ "$pcb"
  kicad-cli pcb export drill -o hardware/build/fab/ "$pcb"
  kicad-cli pcb export step --subst-models -f -o hardware/build/board.step "$pcb"
fi
sch=hardware/generated/board.kicad_sch
if [ -f "$sch" ]; then
  kicad-cli sch export pdf -o hardware/build/schematic.pdf "$sch"
fi
cp hardware/generated/bom.csv hardware/generated/assembly.md hardware/build/

if [ "${SKIP_RENDER:-0}" != "1" ]; then
  node hardware/render.mjs
fi
echo "done: hardware/build/ (review pack in hardware/build/review/)"
