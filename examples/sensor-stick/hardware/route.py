#!/usr/bin/python3
"""hardware/generated/board.kicad_pcb → a routed, poured, DRC-checked board.

    /usr/bin/python3 hardware/route.py

Platform-owned (installed by `fid add capability hardware`). Runs with the
Python KiCad's own `pcbnew` module is built for — the system one, hence the
shebang — not a virtualenv.

fid-hardware derives the board with every part placed and every pad on its
net, and stops there: routing is a search, not a derivation. This does it:

1. design rules: `board.track_mm` (0.2) tracks, `board.clearance_mm` (0.15), 0.5/0.25 mm vias, and wider
   tracks on the nets that carry current (GND and `board.power_nets`);
2. the board out to Specctra DSN, routed by Freerouting (pinned, fetched once
   into hardware/build/tools/ and checked by hash), and read back;
3. a ground pour on both layers, filled;
4. KiCad's own DRC.

With `board.routing = "hand"` it routes nothing: the product's own
hardware/board-routed.kicad_pcb, which `fid derive` has checked against the
declaration, gets the same rules, zone fill and DRC.

It writes hardware/build/board-routed.kicad_pcb (open it in KiCad — this is
the board to fabricate), route.json (the verdict, with the placed board's
and the routed copper's hashes: Freerouting is not deterministic, so a
re-route of the same placement can differ, and this shows it), and copper.json (tracks,
vias, pours and pads, for the review renders). It fails when a net is left
unrouted or DRC finds an error: a board that is not connected is not a board.
"""

from __future__ import annotations

import hashlib
import json
import math
import re
import subprocess
import sys
import urllib.request
from pathlib import Path

import pcbnew

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "hardware" / "generated" / "board.kicad_pcb"
HAND = ROOT / "hardware" / "board-routed.kicad_pcb"  # board.routing = "hand"
BUILD = ROOT / "hardware" / "build"
TOOLS = BUILD / "tools"
FREEROUTING = "2.1.0"
FREEROUTING_URL = f"https://github.com/freerouting/freerouting/releases/download/v{FREEROUTING}/freerouting-{FREEROUTING}.jar"
FREEROUTING_SHA256 = None  # filled on first fetch into tools/freerouting.sha256; see fetch()
MM = pcbnew.FromMM
# Nets that carry current get wider copper: GND, and whatever the product
# names in `board.power_nets` (each a net name or a prefix ending in `*`).
POWER_WIDTH = 0.3
ATTEMPTS = 4  # fresh routing runs before an open connection is the verdict
HOLE_CLEARANCE = 0.2


def board_decl() -> dict:
    import tomllib

    return tomllib.loads((ROOT / "hardware" / "product.toml").read_text()).get("board", {})


def power_nets() -> list[str]:
    return ["GND"] + list(board_decl().get("power_nets", []))


def fetch() -> Path:
    """Freerouting, once: downloaded, hashed, and the hash kept beside it so a
    swapped jar is caught on the next run."""
    TOOLS.mkdir(parents=True, exist_ok=True)
    jar = TOOLS / f"freerouting-{FREEROUTING}.jar"
    pin = TOOLS / f"freerouting-{FREEROUTING}.sha256"
    if not jar.exists():
        with urllib.request.urlopen(FREEROUTING_URL, timeout=120) as r:
            jar.write_bytes(r.read())
    digest = hashlib.sha256(jar.read_bytes()).hexdigest()
    if pin.exists() and pin.read_text().strip() != digest:
        raise SystemExit(f"{jar} does not match its recorded hash — delete tools/ to refetch")
    pin.write_text(digest + "\n")
    return jar


def rules(board: pcbnew.BOARD) -> None:
    # `board.track_mm` and `board.clearance_mm`: 0.2 / 0.15 by default. A
    # 1.27 mm-pitch test pad array (Tag-Connect) or a 0.5 mm QFN escapes only
    # at 0.15 / 0.127 — JLCPCB's standard 2-layer process holds 0.127 / 0.127.
    decl = board_decl()
    track, clear = float(decl.get("track_mm", 0.2)), float(decl.get("clearance_mm", 0.15))
    ds = board.GetDesignSettings()
    ds.m_TrackMinWidth = MM(min(track, 0.15))
    ds.m_MinClearance = MM(clear)
    # Copper to a non-plated hole's edge: 0.2 mm, JLCPCB's standard.
    ds.m_HoleClearance = MM(HOLE_CLEARANCE)
    # Fine-pitch QFN pads sit closer than any mask web a fab can hold;
    # JLCPCB gangs them, so a bridged mask between them is not an error.
    ds.m_SolderMaskMinWidth = 0
    # Vias: 0.5 mm on a 0.25 mm drill, inside JLCPCB's standard 2-layer
    # capability (0.2 mm drill, 0.45 mm via) — small enough to escape a QFN.
    ds.m_MinThroughDrill = MM(0.25)
    ds.m_ViasMinSize = MM(0.5)
    nc = ds.m_NetSettings.m_DefaultNetClass
    nc.SetTrackWidth(MM(track))
    nc.SetClearance(MM(clear))
    nc.SetViaDiameter(MM(0.5))
    nc.SetViaDrill(MM(0.25))
    power = pcbnew.NETCLASS("Power")
    power.SetTrackWidth(MM(POWER_WIDTH))
    power.SetClearance(MM(max(clear, 0.15)))
    power.SetViaDiameter(MM(0.6))
    power.SetViaDrill(MM(0.3))
    ds.m_NetSettings.m_NetClasses["Power"] = power
    for name, net in board.GetNetsByName().items():
        n = str(name)
        if n and any(n == p or (p.endswith("*") and n.startswith(p[:-1])) for p in power_nets()):
            net.SetNetClass(power)


def route(board: pcbnew.BOARD) -> None:
    """Route; while a connection is left open, route again from scratch, and
    keep the best. Freerouting does not give the same answer twice on a dense
    fine-pitch board, so one run that leaves a pad open is not the verdict."""
    # Freerouting keeps only its track clearance from the board edge; KiCad's
    # rule is the edge clearance. A no-track strip of that width, inside the
    # edge, makes the router honour it; it is removed once routing is done.
    # Four plain strips, not one frame with a hole: Freerouting 2.1's maze
    # search fails on a keep-out with a window (NullPointerException, and a
    # third of the connections left open on the same board).
    # One strip per edge of the board's real outline — four for a rectangle,
    # more for a board cut to its case (board.cut_mm) — each straddling its
    # edge, so the inner half keeps tracks off it.
    edge = MM(0.5)
    outline = pcbnew.SHAPE_POLY_SET()
    board.GetBoardPolygonOutlines(outline)
    ring = outline.Outline(0)
    pts = [(ring.CPoint(k).x, ring.CPoint(k).y) for k in range(ring.PointCount())]
    strips = []
    for k in range(len(pts)):
        (ax, ay), (bx, by) = pts[k], pts[(k + 1) % len(pts)]
        length = math.hypot(bx - ax, by - ay)
        if length < 1:
            continue
        nx, ny = -(by - ay) / length * edge, (bx - ax) / length * edge
        quad = [(ax + nx, ay + ny), (bx + nx, by + ny), (bx - nx, by - ny), (ax - nx, ay - ny)]
        strip = pcbnew.ZONE(board)
        strip.SetIsRuleArea(True)
        strip.SetDoNotAllowTracks(True)
        strip.SetDoNotAllowVias(True)
        strip.SetDoNotAllowCopperPour(False)
        strip.SetDoNotAllowPads(False)
        strip.SetDoNotAllowFootprints(False)
        strip.SetLayerSet(pcbnew.LSET.AllCuMask())
        o = strip.Outline()
        o.NewOutline()
        for x, y in quad:
            o.Append(int(round(x)), int(round(y)))
        board.Add(strip)
        strips.append(strip)
    best: tuple[int, str] | None = None
    try:
        for attempt in range(ATTEMPTS):
            if attempt:
                for t in list(board.GetTracks()):
                    board.Remove(t)
            session = route_once(board)
            n = open_connections(board)
            print(f"route attempt {attempt + 1}: {n} connection(s) open")
            if best is None or n < best[0]:
                best = (n, session)
            if n == 0:
                return
        read_session(board, best[1])
    finally:
        for strip in strips:
            board.Remove(strip)


def open_connections(board: pcbnew.BOARD) -> int:
    board.BuildConnectivity()
    return board.GetConnectivity().GetUnconnectedCount(False)


def route_once(board: pcbnew.BOARD) -> str:
    dsn, ses = BUILD / "board.dsn", BUILD / "board.ses"
    ses.unlink(missing_ok=True)
    if not pcbnew.ExportSpecctraDSN(board, str(dsn)):
        raise SystemExit("KiCad could not export the board to Specctra DSN")
    unpad_holes(dsn, board)
    margin_clearances(dsn)
    jar = fetch()
    # Headless, no usage analytics, and a time budget per attempt.
    #
    # Fan-out first: a short escape from every SMD pad before the maze search.
    # Without it, a 0.4 mm-pitch QFN beside its flash left one to four
    # connections open in about half of all runs — the example's CI failed
    # whenever four attempts in a row did. With it, eight runs of eight routed
    # completely, in a quarter of the time.
    cmd = ["java", "-Djava.awt.headless=true", "-jar", str(jar), "--gui.enabled=false",
           "--usage_and_diagnostic_data.disable_analytics=true", "--router.fanout.enabled=true",
           "--router.optimizer.enabled=false", "--router.max_passes=200", "--router.job_timeout=00:02:00",
           "-de", str(dsn), "-do", str(ses), "-mt", "1"]
    p = subprocess.run(cmd, capture_output=True, text=True, timeout=900)
    (BUILD / "freerouting.log").write_text(p.stdout + p.stderr)
    if not ses.exists() or ses.stat().st_size == 0:
        raise SystemExit(f"Freerouting wrote no session; see {BUILD / 'freerouting.log'}")
    text = ses.read_text()
    read_session(board, text)
    return text


def margin_clearances(dsn: Path) -> None:
    """Freerouting routes at the clearance exactly, and its read-back can land
    a hair under it: one route in four of the same placement once came back
    with a track 0.1252 mm from a pad, against a 0.127 mm rule, and failed
    DRC. The router gets 10 µm more than the rule; KiCad's DRC checks the
    rule itself."""
    text = dsn.read_text()
    if "(unit um)" not in text and "(resolution um" not in text:
        return
    bump = lambda m: f"(clearance {float(m.group(1)) + 10:.1f}" if float(m.group(1)) >= 100 else m.group(0)  # noqa: E731
    dsn.write_text(re.sub(r"\(clearance ([\d.]+)", bump, text))


def unpad_holes(dsn: Path, board: pcbnew.BOARD) -> None:
    """KiCad exports a non-plated hole as a keep-out of the drill plus the hole
    clearance, and Freerouting then keeps its own track clearance from that —
    the margin counted twice. Between a Tag-Connect's leg holes that closes a
    0.8 mm gap a 0.15 mm track fits with room to spare. Take the router's
    clearance back off each hole keep-out; KiCad's DRC still checks the real one."""
    text = dsn.read_text()
    if "(unit um)" not in text and "(resolution um" not in text:
        return
    clear_um = pcbnew.ToMM(board.GetDesignSettings().m_NetSettings.m_DefaultNetClass.GetClearance()) * 1000
    shrink = lambda m: f'(keepout "" (circle {m.group(1)} {max(float(m.group(2)) - 2 * clear_um, 1):.1f}'  # noqa: E731
    dsn.write_text(re.sub(r'\(keepout "" \(circle (\S+) ([\d.]+)', shrink, text))


def sexpr(text: str):
    """A Specctra file as nested lists of atoms."""
    stack, cur, tok, quoted = [], [], "", False
    for ch in text:
        if quoted:
            if ch == '"':
                quoted = False
                cur.append(tok)
                tok = ""
            else:
                tok += ch
        elif ch == '"':
            quoted = True
        elif ch in "() \t\r\n":
            if tok:
                cur.append(tok)
                tok = ""
            if ch == "(":
                stack.append(cur)
                cur = []
            elif ch == ")":
                done, cur = cur, stack.pop()
                cur.append(done)
        else:
            tok += ch
    return cur[0]


def read_session(board: pcbnew.BOARD, text: str) -> None:
    """Freerouting's session (.ses) → tracks and vias on the board.

    KiCad 7's ImportSpecctraSES works only inside the editor, so the session
    is read here: each `wire` path is a run of tracks on its layer, each `via`
    a via whose padstack name carries its diameter and drill (µm).
    """
    tree = sexpr(text)
    routes = next(x for x in tree if isinstance(x, list) and x and x[0] == "routes")
    # The session holds the whole routing, earlier passes included.
    for t in list(board.GetTracks()):
        board.Remove(t)
    res = next(x for x in routes if isinstance(x, list) and x and x[0] == "resolution")
    per_mm = {"um": 1000.0, "mm": 1.0, "mil": 1000 / 25.4}[res[1]] * float(res[2])
    # The DSN KiCad wrote has y pointing up.
    to = lambda x, y: pcbnew.VECTOR2I(MM(float(x) / per_mm), MM(-float(y) / per_mm))
    layers = {board.GetLayerName(l): l for l in (pcbnew.F_Cu, pcbnew.B_Cu)}
    net_out = next(x for x in routes if isinstance(x, list) and x and x[0] == "network_out")
    for net in net_out[1:]:
        code = board.FindNet(net[1])
        for item in net[2:]:
            if item[0] == "wire":
                path = next(x for x in item[1:] if isinstance(x, list) and x[0] == "path")
                layer, width, pts = path[1], float(path[2]) / per_mm, path[3:]
                xy = [to(pts[i], pts[i + 1]) for i in range(0, len(pts) - 1, 2)]
                for a, b in zip(xy, xy[1:]):
                    t = pcbnew.PCB_TRACK(board)
                    t.SetStart(a)
                    t.SetEnd(b)
                    t.SetWidth(MM(width))
                    t.SetLayer(layers[layer])
                    t.SetNet(code)
                    board.Add(t)
            elif item[0] == "via":
                dims = item[1].split("_")[-2].split(":")
                v = pcbnew.PCB_VIA(board)
                v.SetPosition(to(item[2], item[3]))
                v.SetWidth(MM(float(dims[0]) / 1000))
                v.SetDrill(MM(float(dims[1]) / 1000))
                v.SetViaType(pcbnew.VIATYPE_THROUGH)
                v.SetLayerPair(pcbnew.F_Cu, pcbnew.B_Cu)
                v.SetNet(code)
                board.Add(v)


def pour(board: pcbnew.BOARD) -> None:
    gnd = board.FindNet("GND")
    if gnd is None:
        return
    box = board.GetBoardEdgesBoundingBox()
    for layer in (pcbnew.F_Cu, pcbnew.B_Cu):
        z = pcbnew.ZONE(board)
        z.SetLayer(layer)
        z.SetNet(gnd)
        z.SetZoneName(f"GND-{board.GetLayerName(layer)}")
        z.SetLocalClearance(MM(0.3))
        z.SetMinThickness(MM(0.25))
        z.SetPadConnection(pcbnew.ZONE_CONNECTION_FULL)  # reflowed, not hand-soldered
        z.SetThermalReliefGap(MM(0.3))
        z.SetThermalReliefSpokeWidth(MM(0.4))
        z.SetAssignedPriority(0)
        # Copper that touches no ground pad or track is an antenna, not a
        # ground: drop those islands rather than leave them floating.
        z.SetIslandRemovalMode(pcbnew.ISLAND_REMOVAL_MODE_ALWAYS)
        o = z.Outline()
        o.NewOutline()
        for x, y in ((box.GetLeft(), box.GetTop()), (box.GetRight(), box.GetTop()),
                     (box.GetRight(), box.GetBottom()), (box.GetLeft(), box.GetBottom())):
            o.Append(x, y)
        board.Add(z)
    pcbnew.ZONE_FILLER(board).Fill(board.Zones())


def drc(board: pcbnew.BOARD, path: Path) -> dict:
    pcbnew.WriteDRCReport(board, str(path), pcbnew.EDA_UNITS_MILLIMETRES, True)
    # Each item: `[kind]: message`, then `Rule…; Severity: error|warning`.
    errors: dict[str, int] = {}
    warnings: dict[str, int] = {}
    # Items are blocks: `[kind]: message`, a rule line with the severity,
    # then the `@(x, y): what` lines it involves.
    blocks: list[list[str]] = []
    for line in path.read_text().splitlines():
        if line.startswith("["):
            blocks.append([line])
        elif blocks and line.startswith("    "):
            blocks[-1].append(line)
    for b in blocks:
        kind = b[0][1:b[0].index("]")]
        severity = next((l for l in b if "Severity:" in l), "")
        # Library-link notices say the footprint differs from a library
        # this board was never linked to: true of every derived board.
        if kind == "lib_footprint_issues" or not severity:
            continue
        error = "Severity: error" in severity
        # A footprint's own mask opening (a graphic on F.Mask, not a pad) over
        # copper is the footprint's design — U.FL ground — not a bridge.
        if kind == "solder_mask_bridge" and any("Rect on F.Mask" in l or "Polygon on F.Mask" in l for l in b):
            error = False
        bucket = errors if error else warnings
        bucket[kind] = bucket.get(kind, 0) + 1
    return {
        "violations": sum(v for k, v in errors.items() if k != "unconnected_items"),
        "unconnected": errors.get("unconnected_items", 0),
        "errors": errors,
        "warnings": warnings,
        "report": str(path.relative_to(ROOT)),
    }


def routed_hash(board: pcbnew.BOARD) -> str:
    """The routed copper — tracks and vias, to the micrometre, in a fixed
    order — hashed. The board file itself carries UUIDs and a fill, so its
    bytes would differ on an identical route."""
    um = lambda v: round(pcbnew.ToMM(v), 3)
    items = []
    for t in board.GetTracks():
        if t.GetClass() == "PCB_VIA":
            items.append(("via", um(t.GetPosition().x), um(t.GetPosition().y), um(t.GetWidth()), t.GetNetname()))
        else:
            a, b = sorted([(um(t.GetStart().x), um(t.GetStart().y)), (um(t.GetEnd().x), um(t.GetEnd().y))])
            items.append(("track", board.GetLayerName(t.GetLayer()), *a, *b, um(t.GetWidth()), t.GetNetname()))
    return hashlib.sha256(json.dumps(sorted(items, key=str)).encode()).hexdigest()


def copper(board: pcbnew.BOARD) -> dict:
    """Everything copper, in KiCad board coordinates (mm, y down)."""
    mm = pcbnew.ToMM
    out = {"tracks": [], "vias": [], "pours": [], "pads": [], "refs": []}
    for t in board.GetTracks():
        if t.GetClass() == "PCB_VIA":
            out["vias"].append({"at": [mm(t.GetPosition().x), mm(t.GetPosition().y)],
                                "d": mm(t.GetWidth()), "drill": mm(t.GetDrillValue())})
        else:
            out["tracks"].append({"layer": board.GetLayerName(t.GetLayer()),
                                  "a": [mm(t.GetStart().x), mm(t.GetStart().y)],
                                  "b": [mm(t.GetEnd().x), mm(t.GetEnd().y)],
                                  "w": mm(t.GetWidth()), "net": t.GetNetname()})
    for z in board.Zones():
        if z.GetIsRuleArea():
            continue
        for layer in (pcbnew.F_Cu, pcbnew.B_Cu):
            if not z.IsOnLayer(layer):
                continue
            polys = z.GetFilledPolysList(layer)
            for i in range(polys.OutlineCount()):
                ring = polys.Outline(i)
                holes = [[[mm(h.CPoint(k).x), mm(h.CPoint(k).y)] for k in range(h.PointCount())]
                         for h in (polys.Hole(i, j) for j in range(polys.HoleCount(i)))]
                out["pours"].append({"layer": board.GetLayerName(layer),
                                     "outline": [[mm(ring.CPoint(k).x), mm(ring.CPoint(k).y)] for k in range(ring.PointCount())],
                                     "holes": holes})
    for fp in board.GetFootprints():
        p = fp.GetPosition()
        out["refs"].append({"ref": fp.GetReference(), "at": [mm(p.x), mm(p.y)], "rot": fp.GetOrientationDegrees(),
                            "layer": board.GetLayerName(fp.GetLayer())})
        for pad in fp.Pads():
            q = pad.GetPosition()
            out["pads"].append({"at": [mm(q.x), mm(q.y)], "size": [mm(pad.GetSize().x), mm(pad.GetSize().y)],
                                "rot": pad.GetOrientationDegrees(),
                                "shape": "circle" if pad.GetShape() == pcbnew.PAD_SHAPE_CIRCLE else "rect",
                                "drill": mm(pad.GetDrillSize().x),
                                "layers": [n for n in ("F.Cu", "B.Cu") if pad.IsOnLayer(board.GetLayerID(n))],
                                "net": pad.GetNetname()})
    return out


def main() -> int:
    BUILD.mkdir(parents=True, exist_ok=True)
    # `board.routing = "hand"`: a person routed it in KiCad, and `fid derive`
    # has already refused it unless its parts, nets and outline are still the
    # declaration's. Nothing to route; the same rules and DRC decide it.
    hand = board_decl().get("routing", "auto") == "hand"
    if hand:
        if not HAND.exists():
            raise SystemExit(f"board.routing = \"hand\" and there is no {HAND.relative_to(ROOT)}")
        board = pcbnew.LoadBoard(str(HAND))
        rules(board)
    else:
        board = pcbnew.LoadBoard(str(SRC))
        rules(board)
        route(board)
        pour(board)
    out = BUILD / "board-routed.kicad_pcb"
    board.Save(str(out))
    board = pcbnew.LoadBoard(str(out))
    pcbnew.ZONE_FILLER(board).Fill(board.Zones())
    verdict = drc(board, BUILD / "drc.rpt")
    verdict["tracks"] = sum(1 for t in board.GetTracks() if t.GetClass() != "PCB_VIA")
    verdict["vias"] = sum(1 for t in board.GetTracks() if t.GetClass() == "PCB_VIA")
    verdict["router"] = f"by hand ({HAND.relative_to(ROOT)})" if hand else f"Freerouting {FREEROUTING}"
    # Placement is deterministic; Freerouting is not. The placed board's hash
    # and the routed copper's, side by side, make a different route visible:
    # same `placed`, different `routed` is the router, not the design.
    verdict["placed_sha256"] = hashlib.sha256(SRC.read_bytes()).hexdigest()
    verdict["routed_sha256"] = routed_hash(board)
    board.Save(str(out))
    (BUILD / "route.json").write_text(json.dumps(verdict, indent=2) + "\n")
    (BUILD / "copper.json").write_text(json.dumps(copper(board)) + "\n")
    ok = verdict["unconnected"] == 0 and verdict["violations"] == 0
    print(f"routed: {verdict['tracks']} tracks, {verdict['vias']} vias "
          f"(copper {verdict['routed_sha256'][:12]}); "
          f"DRC {verdict['violations']} violation(s), {verdict['unconnected']} unconnected"
          + ("" if ok else f" — see {verdict['report']}"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
