#!/usr/bin/env python3
"""The routed board → what a board house needs to make and assemble it.

    python3 hardware/fab.py

Platform-owned (installed by `fid add capability hardware`). No dependencies
beyond kicad-cli. `build.sh` runs it after routing.

Writes hardware/build/fab/ from scratch every time — a stale Gerber from an
earlier board beside the current one is how the wrong board gets ordered:

- the Gerbers and drill file, and `gerbers.zip` of them (the upload);
- `bom-assembly.csv` — Comment, Designator, Footprint, LCSC Part #: the
  column set JLCPCB's assembly service reads, one row per distinct part,
  every designator it is placed at;
- `cpl.csv` — Designator, Mid X, Mid Y, Layer, Rotation: where each part
  goes, from KiCad's own position export of the routed board;
- `README.md` — what each file is, and every board part the assembly
  service cannot source because it has no `lcsc` declared. That list is the
  honest answer to "can I order this today?".

Part rotations are KiCad's. Board houses sometimes define a footprint's zero
rotation differently; check the assembly preview before ordering.
"""

from __future__ import annotations

import csv
import json
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BUILD = ROOT / "hardware" / "build"
FAB = BUILD / "fab"
GENERATED = ROOT / "hardware" / "generated"


def assembled(p: dict) -> bool:
    """Something to place and solder: wire pads and a pads-only footprint
    (a programming clip's landing, say) have nothing on them to buy."""
    if p.get("mount") == "pads":
        return False
    fid = (p.get("footprint") or {}).get("id")
    if not fid:
        return True
    lib = ROOT / "hardware" / "lib" / "footprints" / (fid.split(":")[-1] + ".kicad_mod")
    if not lib.exists():
        return True
    text = lib.read_text()
    return "F.Paste" in text or "B.Paste" in text or "thru_hole" in text


def run(*args: str) -> None:
    subprocess.run(args, check=True, stdout=subprocess.DEVNULL)


def main() -> int:
    pcb = BUILD / "board-routed.kicad_pcb"
    if not pcb.exists():
        pcb = GENERATED / "board.kicad_pcb"
    if not pcb.exists():
        print("no board to fabricate", file=sys.stderr)
        return 0
    shutil.rmtree(FAB, ignore_errors=True)
    FAB.mkdir(parents=True)
    run("kicad-cli", "pcb", "export", "gerbers", "-o", f"{FAB}/", str(pcb))
    run("kicad-cli", "pcb", "export", "drill", "-o", f"{FAB}/", str(pcb))
    with zipfile.ZipFile(FAB / "gerbers.zip", "w", zipfile.ZIP_DEFLATED) as z:
        for f in sorted(FAB.iterdir()):
            if f.suffix in (".gtl", ".gbl", ".gto", ".gbo", ".gts", ".gbs", ".gtp", ".gbp", ".gm1", ".drl", ".gbrjob"):
                z.write(f, f.name)

    layout = json.loads((GENERATED / "layout.json").read_text())
    placed = layout.get("board", {}).get("illustrative_placement", [])
    on_bom = {p.get("ref", p["part"]) for p in placed if assembled(p)}

    # Where every part sits, from KiCad's own export of the routed board.
    pos = BUILD / "positions.csv"
    run("kicad-cli", "pcb", "export", "pos", "--format", "csv", "--units", "mm", "--side", "both", "-o", str(pos), str(pcb))
    with pos.open() as f, (FAB / "cpl.csv").open("w", newline="") as out:
        w = csv.writer(out)
        w.writerow(["Designator", "Mid X", "Mid Y", "Layer", "Rotation"])
        for row in csv.DictReader(f):
            if row["Ref"] not in on_bom:
                continue
            w.writerow([row["Ref"], f'{float(row["PosX"]):.3f}mm', f'{float(row["PosY"]):.3f}mm',
                        "Top" if row["Side"].lower().startswith("top") else "Bottom", f'{float(row["Rot"]):.1f}'])

    # One row per declared board part: its value, every designator, its LCSC number.
    bom = {r["id"]: r for r in csv.DictReader((GENERATED / "bom.csv").open())}
    refs: dict[str, list[str]] = {}
    values: dict[str, str] = {}
    for p in placed:
        if assembled(p):
            refs.setdefault(p["part"], []).append(p.get("ref", p["part"]))
            if p.get("value"):
                values[p["part"]] = p["value"]
    unsourced = []
    with (FAB / "bom-assembly.csv").open("w", newline="") as out:
        w = csv.writer(out)
        w.writerow(["Comment", "Designator", "Footprint", "LCSC Part #"])
        for pid, designators in sorted(refs.items()):
            line = bom.get(pid, {})
            value = values.get(pid) or line.get("mpn") or line.get("name") or pid
            fp = (line.get("footprint") or "").split(":")[-1]
            lcsc = line.get("lcsc", "")
            if not lcsc:
                unsourced.append(f"`{pid}` ({', '.join(designators)}) — {line.get('name', '')}")
            w.writerow([value, ",".join(designators), fp, lcsc])

    readme = [
        "# Fabrication files",
        "",
        f"Derived from `{pcb.relative_to(ROOT)}` by `hardware/fab.py`. Rewritten on every build.",
        "",
        "| File | For |",
        "|---|---|",
        "| `gerbers.zip` | the board: upload this for PCB fabrication |",
        "| `bom-assembly.csv` | assembly: Comment, Designator, Footprint, LCSC Part # |",
        "| `cpl.csv` | assembly: where each part goes (Designator, Mid X, Mid Y, Layer, Rotation) |",
        "",
    ]
    if unsourced:
        readme += [
            f"## Not orderable for assembly yet: {len(unsourced)} part(s) without an `lcsc` number",
            "",
            "Declare `lcsc` on each in `hardware/product.toml` (or fit them by hand):",
            "",
            *[f"- {u}" for u in unsourced],
            "",
        ]
    else:
        readme += ["Every board part has an LCSC number: the BOM can be ordered for assembly as it stands.", ""]
    readme += ["Rotations are KiCad's; check the board house's assembly preview before ordering."]
    (FAB / "README.md").write_text("\n".join(readme) + "\n")
    print(f"wrote hardware/build/fab/ — gerbers.zip, bom-assembly.csv, cpl.csv; {len(unsourced)} part(s) without lcsc")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
