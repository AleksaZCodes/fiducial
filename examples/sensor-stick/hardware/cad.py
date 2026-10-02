#!/usr/bin/env python3
"""hardware/generated/layout.json -> a functional, assemblable model.

    pip install -r hardware/requirements.txt
    python3 hardware/cad.py              # writes hardware/build/

Platform-owned (installed by `fid add capability hardware`, overwritten by
`fid upgrade`). It makes **no decisions**: every position and size comes from
layout.json, which `fid-hardware` solved and `fid derive --check` gates. What
this file adds is what only a solid kernel can: offsetting a non-convex
outline, routing a gasket groove round the screw bosses, and proving the thing
can be put together.

It fails (exit 1) unless all of these hold, and writes the verdict to
build/checks.json either way:

  interference   no two solids overlap in the assembled pose
  insertion      every part dropped into the base clears the base on its way down
  lid            the lid closes onto the parts without touching them
  screws         every screw engages at least 1.5 diameters of plastic
  seal           the lid presses into the gasket — at least its protrusion,
                 at most its declared compression; a gasket nothing reaches
                 seals nothing
  models         a part drawn from its own STEP (`model =`) fits the
                 envelope it declared, which is what the layout was solved for

The product's own geometry: hardware/shapes.py, when it exists, is called
with the built solids before any check runs — `shapes(layout, solids, kit)` —
and may add solids, or cut and join the ones there. It is the product's file,
never installed or overwritten by fiducial, and everything it makes is
checked exactly like everything this file makes: a hand-written bracket that
collides with the board fails interference by name. What it added, removed
and changed is listed in checks.json under `custom`.

Outputs, in hardware/build/:
  <part>.step    every solid, for CAD and for a manufacturer
  <part>.stl     the printed parts and every solid, for slicing and rendering
  assembly.step  the whole product, positioned
  scene.json     what the renderer draws: part, file, look, assembled/exploded offsets
  checks.json    the four checks above, and their evidence
"""

from __future__ import annotations

import importlib.util
import json
import math
import sys
from pathlib import Path
from types import SimpleNamespace

from build123d import (
    Align,
    Axis,
    Box,
    Compound,
    Cylinder,
    Face,
    GeomType,
    Kind,
    Location,
    Part,
    Plane,
    Polyline,
    Pos,
    Rot,
    Solid,
    Sphere,
    Wire,
    chamfer,
    export_step,
    import_step,
    export_stl,
    extrude,
    make_face,
    offset,
)

ROOT = Path(__file__).resolve().parent.parent
LAYOUT = ROOT / "hardware" / "generated" / "layout.json"
BUILD = ROOT / "hardware" / "build"
TOL = 1e-3  # mm³ — below this, two touching faces, not an overlap


def face(pts):
    return make_face(Polyline(*[tuple(p) for p in pts], close=True))


def slab(sketch, z0, h):
    """Extrude a 2D sketch from z0 up by h."""
    return Pos(0, 0, z0) * extrude(sketch, amount=h)


def disc(c, r):
    return Pos(c[0], c[1], 0) * make_face(Polyline(*[
        (r * math.cos(2 * math.pi * i / 48), r * math.sin(2 * math.pi * i / 48)) for i in range(48)
    ], close=True))


def box(c, size, z0, h):
    return Pos(c[0], c[1], z0) * Box(size[0], size[1], h, align=(Align.CENTER, Align.CENTER, Align.MIN))


def cyl(c, r, z0, h):
    return Pos(c[0], c[1], z0) * Cylinder(r, h, align=(Align.CENTER, Align.CENTER, Align.MIN))


def lean(c0, c1, r, z0, z1):
    """A cylinder from a circle round c0 at z0 to one round c1 at z1: it leans."""
    loft = Solid.make_loft([Wire.make_circle(r, Plane(origin=(c0[0], c0[1], z0))), Wire.make_circle(r, Plane(origin=(c1[0], c1[1], z1)))])
    return Part(loft.wrapped)  # a Part, as everything else here: & with nothing is empty, not None


COPPER = 0.035  # mm: 1 oz copper, the pads a part sits on

# Faces of a model, grouped by their colour, for the renderer: a KiCad model
# is one solid whose faces carry the body's black and the leads' tin. Checks
# use the solid; the picture uses these. name -> [(shape, "#rrggbb")].
RENDERS: dict = {}


def step_by_colour(path):
    """A STEP model's faces, grouped by the colour the file gives them.

    KiCad's models colour faces (body, leads, marking); an assembly colours
    its parts. Both are read here, through OpenCascade's XCAF document, so a
    part renders as the manufacturer's model does, not as one grey mass.
    """
    from collections import defaultdict

    from OCP.Quantity import Quantity_ColorRGBA, Quantity_TOC_sRGB
    from OCP.STEPCAFControl import STEPCAFControl_Reader
    from OCP.TCollection import TCollection_ExtendedString
    from OCP.TDF import TDF_Label
    from OCP.TDocStd import TDocStd_Document
    from OCP.TopAbs import TopAbs_FACE
    from OCP.TopExp import TopExp_Explorer
    from OCP.TopLoc import TopLoc_Location
    from OCP.XCAFDoc import XCAFDoc_ColorCurv, XCAFDoc_ColorGen, XCAFDoc_ColorSurf, XCAFDoc_ColorTool, XCAFDoc_DocumentTool, XCAFDoc_ShapeTool
    import build123d.importers as imp

    def hexof(c):
        r, g, b = c.GetRGB().Values(Quantity_TOC_sRGB)
        return "#%02x%02x%02x" % tuple(max(0, min(255, round(v * 255))) for v in (r, g, b))

    doc = TDocStd_Document(TCollection_ExtendedString("XmlOcaf"))
    rd = STEPCAFControl_Reader()
    rd.SetColorMode(True)
    rd.ReadFile(str(path))
    rd.Transfer(doc)
    st = XCAFDoc_DocumentTool.ShapeTool_s(doc.Main())
    ct = XCAFDoc_DocumentTool.ColorTool_s(doc.Main())
    groups = defaultdict(list)

    def label_colour(lab):
        c = Quantity_ColorRGBA()
        for t in (XCAFDoc_ColorSurf, XCAFDoc_ColorGen):
            if XCAFDoc_ColorTool.GetColor_s(lab, t, c):
                return hexof(c)
        return None

    def walk(lab, loc, inherited):
        if XCAFDoc_ShapeTool.IsReference_s(lab):
            ref = TDF_Label()
            XCAFDoc_ShapeTool.GetReferredShape_s(lab, ref)
            loc = loc.Multiplied(XCAFDoc_ShapeTool.GetLocation_s(lab))
            return walk(ref, loc, label_colour(lab) or label_colour(ref) or inherited)
        col = label_colour(lab) or inherited
        if XCAFDoc_ShapeTool.IsAssembly_s(lab):
            kids = imp.Sequence_TDF_Label()
            XCAFDoc_ShapeTool.GetComponents_s(lab, kids)
            for i in range(kids.Length()):
                walk(kids.Value(i + 1), loc, col)
            return
        ex = TopExp_Explorer(XCAFDoc_ShapeTool.GetShape_s(lab), TopAbs_FACE)
        while ex.More():
            f, c = ex.Current(), Quantity_ColorRGBA()
            got = any(ct.GetColor(f, t, c) for t in (XCAFDoc_ColorSurf, XCAFDoc_ColorGen, XCAFDoc_ColorCurv))
            groups[hexof(c) if got else (col or "#3a3a3e")].append(Face(f.Moved(loc)))
            ex.Next()

    free = imp.Sequence_TDF_Label()
    st.GetFreeShapes(free)
    for i in range(free.Length()):
        walk(free.Value(i + 1), TopLoc_Location(), None)
    return [(Compound(v), k) for k, v in groups.items()]


def package(fp, rot, top, h):
    """A body built from the footprint, for a part no library has a model of:
    the fab outline, at the declared height, in epoxy, with a pin-1 dot — so a
    QFN looks like a QFN and an inductor like an inductor, not a placeholder."""
    (cx, cy), (w, d) = fp["fab"]["centre_mm"], fp["fab"]["size_mm"]
    body = Pos(cx, cy, top) * Box(w, d, h, align=(Align.CENTER, Align.CENTER, Align.MIN))
    body = chamfer(body.edges().group_by(Axis.Z)[-1], min(0.15, h / 4)) if h > 0.3 else body
    pieces = [(body, "#232326")]
    pin1 = next((p for p in fp["pads"] if p["pin"] == "1"), None)
    if pin1 and h > 0.3:
        # Inboard of pad 1, on the top face.
        px, py = pin1["at_mm"]
        k = 0.65
        dot = Pos(cx + (px - cx) * k, cy + (py - cy) * k, top + h) * Cylinder(min(w, d) * 0.06, 0.02, align=(Align.CENTER, Align.CENTER, Align.MIN))
        pieces.append((dot, "#bdbdbd"))
    return body, pieces


def board_part(solids, nm, q, top, look, routed):
    """One part on the board, as it will be built.

    A part with a KiCad footprint gets its STEP model — the manufacturer's
    body, in its own colours — or, where no library ships one, a body built
    from the footprint itself. Its copper is drawn from the routed board when
    there is one (route.py), from its pads when not. Anything else is its
    declared envelope.
    """
    rot = q.get("rotation_deg", 0.0)
    fp = q.get("footprint")
    if q["mount"] == "pads":
        if routed:
            return  # the routed board's copper draws them
        (cx, cy), n = q["body"]["centre_mm"], len(q["nets"])
        pw, ph = q["pad_mm"]
        a = math.radians(rot)
        for i in range(n):
            u = (i - (n - 1) / 2) * q["pitch_mm"]
            c = (cx + u * math.cos(a), cy + u * math.sin(a))
            solids[f"{nm}-copper-{i}"] = (Pos(c[0], c[1], top) * Rot(0, 0, rot) * Box(pw, ph, COPPER, align=(Align.CENTER, Align.CENTER, Align.MIN)), "pin", "board")
        return
    if fp:
        if not routed:
            copper = None
            for pad in fp["pads"]:
                if not pad["pin"]:
                    continue  # a paste aperture, not copper
                shape = box(pad["at_mm"], pad["size_mm"], top, COPPER)
                copper = shape if copper is None else copper + shape
            if copper is not None:
                solids[f"{nm}-copper"] = (copper, "pin", "board")
        m = fp.get("model")
        path = ROOT / m["file"] if m else None
        if path and path.exists():
            rx, ry, rz = m["rotate_deg"]
            ox, oy, oz = m["offset_mm"]
            (x, y) = fp["origin_mm"]
            place = lambda s: Pos(x, y, top) * Rot(0, 0, rot) * Pos(ox, oy, oz) * Rot(rx, ry, rz) * s
            solids[nm] = (place(import_step(str(path))), "component", "board")
            RENDERS[nm] = [(place(s), c) for s, c in step_by_colour(path)]
            return
        if fp.get("fab") and q["height_mm"] > 0.1:
            body, pieces = package(fp, rot, top, q["height_mm"])
            solids[nm] = (body, "component", "board")
            RENDERS[nm] = pieces
            return
        if q["height_mm"] <= 0.1:
            return  # pads only (a Tag-Connect footprint): the copper is the part
    solids[nm] = (box(q["body"]["centre_mm"], q["body"]["size_mm"], top, q["height_mm"]), look, "board")



def opening(op):
    """The window through the wall in front of a board part's mouth: from just
    inside the mouth, out past the outer face, its long side horizontal."""
    (x, y, z), (nx, ny) = op["mouth_mm"], op["normal"]
    w, h = op["size_mm"]
    pl = Plane(origin=(x - nx * 0.5, y - ny * 0.5, z), x_dir=(-ny, nx, 0), z_dir=(nx, ny, 0))
    return pl * Box(w, h, op["length_mm"] + 0.5, align=(Align.CENTER, Align.CENTER, Align.MIN))


def apply_models(L, solids, out):
    """Each part with `model =` drawn from its own STEP, in place of its
    envelope: turned by `rotate_deg`, centred where the envelope was, standing
    on the envelope's floor. A model larger than the envelope fails — the
    layout was solved for the envelope, so a bigger part was never placed."""
    for pid, m in L.get("models", {}).items():
        names = [n for n in solids if n == pid or (n.startswith(pid + "-") and n[len(pid) + 1:].isdigit())]
        for nm in names:
            old, look, group = solids[nm]
            ob = old.bounding_box()
            rx, ry, rz = m["rotate_deg"]
            raw = Rot(rx, ry, rz) * import_step(str(ROOT / m["file"]))
            rb = raw.bounding_box()
            move = Pos((ob.min.X + ob.max.X) / 2 - (rb.min.X + rb.max.X) / 2,
                       (ob.min.Y + ob.max.Y) / 2 - (rb.min.Y + rb.max.Y) / 2,
                       ob.min.Z - rb.min.Z)
            over = [round(a - b, 3) for a, b in ((rb.size.X, ob.size.X), (rb.size.Y, ob.size.Y), (rb.size.Z, ob.size.Z))]
            if any(o > 0.5 for o in over):
                out["models"].append({
                    "part": nm, "file": m["file"],
                    "problem": "the model is larger than the envelope the layout was solved for — declare body_mm to fit it",
                    "model_mm": [round(rb.size.X, 2), round(rb.size.Y, 2), round(rb.size.Z, 2)],
                    "envelope_mm": [round(ob.size.X, 2), round(ob.size.Y, 2), round(ob.size.Z, 2)],
                })
            solids[nm] = (move * raw, look, group)
            pieces = [(move * Rot(rx, ry, rz) * piece, colour) for piece, colour in step_by_colour(ROOT / m["file"])]
            if pieces:
                RENDERS[nm] = pieces  # its own colours; an uncoloured model renders in the part's look


def kit():
    """What hardware/shapes.py gets beside the solids: the same helpers this
    file builds with, and build123d itself for anything they do not cover."""
    import build123d

    return SimpleNamespace(box=box, cyl=cyl, face=face, slab=slab, disc=disc, opening=opening,
                           bd=build123d, looks=sorted(LOOK), groups=sorted(EXPLODE))


def product_shapes(L, solids):
    """Run the product's hardware/shapes.py, if it has one, and say what it did."""
    hook = ROOT / "hardware" / "shapes.py"
    if not hook.exists():
        return None
    spec = importlib.util.spec_from_file_location("product_shapes", hook)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    if not hasattr(mod, "shapes"):
        raise SystemExit("hardware/shapes.py defines no shapes(layout, solids, kit) function")
    before = {n: s.volume for n, (s, _, _) in solids.items()}
    mod.shapes(L, solids, kit())
    for keep in ("case-base", "case-lid"):
        if keep not in solids:
            raise SystemExit(f"hardware/shapes.py removed `{keep}`: cut and join it, but it must stay")
    for nm, entry in solids.items():
        if not (isinstance(entry, tuple) and len(entry) == 3):
            raise SystemExit(f"hardware/shapes.py: solids[{nm!r}] must be (shape, look, group)")
        _, look, group = entry
        if group not in EXPLODE:
            raise SystemExit(f"hardware/shapes.py: `{nm}` is in group `{group}`; one of {', '.join(sorted(EXPLODE))}")
        if look not in LOOK and not str(look).startswith("#"):
            raise SystemExit(f"hardware/shapes.py: `{nm}` has look `{look}`; one of {', '.join(sorted(LOOK))}, or a #rrggbb colour")
    return {
        "added": sorted(set(solids) - set(before)),
        "removed": sorted(set(before) - set(solids)),
        "changed": sorted(n for n in before if n in solids and abs(solids[n][0].volume - before[n]) > TOL),
    }


def l_locators(body, clr, t, leg, z0, h, leads=None):
    """Four L-shaped ribs hugging a rectangle's corners, `clr` off its faces.

    `leads`: (axis, sign, span) — the end the part's leads come out of, and
    how wide they spread. The ribs across that end stop short of them, so
    the leads are not built into the plastic."""
    (cx, cy), (w, d) = body["centre_mm"], body["size_mm"]
    out = None
    for sx in (-1, 1):
        for sy in (-1, 1):
            x = cx + sx * (w / 2 + clr + t / 2)
            y = cy + sy * (d / 2 + clr + t / 2)
            la = lb = leg
            if leads:
                axis, sign, span = leads
                if axis == "x" and sx == sign:      # rib `a` runs across the x end
                    la = max(t, min(leg, d / 2 + clr - span / 2 - 1.0))
                if axis == "y" and sy == sign:      # rib `b` runs across the y end
                    lb = max(t, min(leg, w / 2 + clr - span / 2 - 1.0))
            a = box((x, y - sy * (la / 2 - t / 2)), (t, la), z0, h)
            b = box((x - sx * (lb / 2 - t / 2), y), (lb, t), z0, h)
            out = a + b if out is None else out + a + b
    return out


def build(L):
    c = L["case"]
    seal = c.get("seal")
    sc = c["screws"]
    H, floor, wall, lid_t, clr = c["base_height_mm"], c["floor_mm"], c["wall_mm"], c["lid_mm"], c["clearance_mm"]
    outline = face(c["outline_mm"])

    closure = c.get("closure", "screws-top")
    bosses = None
    for p in sc["at_mm"]:
        d = disc(p, sc["boss_radius_mm"])
        bosses = d if bosses is None else bosses + d
    cavity = offset(outline, amount=-wall, kind=Kind.ARC)
    if bosses is not None:
        cavity = cavity - bosses
    hg = c.get("hang")
    if hg and hg.get("kind") == "holes":
        # The tip is solid: the cavity, and with it the seal, stop short of it.
        xs = [p[0] for p in c["outline_mm"]]
        ys = [p[1] for p in c["outline_mm"]]
        py = hg["plug_y_mm"]
        cavity = cavity - face([(min(xs) - 5, py), (max(xs) + 5, py), (max(xs) + 5, max(ys) + 5), (min(xs) - 5, max(ys) + 5)])

    solids = {}   # name -> (solid, look, group)

    # ── Base ────────────────────────────────────────────────────────────────
    base = slab(outline, 0, H) - slab(cavity, floor, H)
    if hg and hg.get("kind") == "lug":
        # A lug out of the top of the outline, on the back, outside the seal:
        # a flat tab with a rounded end and a hole to hang the case by.
        (ax, ay), (hx, hy) = hg["at_mm"], hg["hole_at_mm"]
        w, t = hg["width_mm"], hg["thickness_mm"]
        tab = box((ax, (ay - 0.5 + hy) / 2), (w, hy - ay + 0.5), 0, t) + cyl((hx, hy), w / 2, 0, t)
        tab -= cyl((hx, hy), hg["hole_mm"] / 2, -1, t + 2)
        base += tab
    if seal:
        band = offset(cavity, amount=seal["lip_mm"] + seal["groove_mm"], kind=Kind.ARC) - offset(
            cavity, amount=seal["lip_mm"], kind=Kind.ARC
        )
        base -= slab(band, H - seal["depth_mm"], seal["depth_mm"] + 1)
    head_d, head_h = sc["head_mm"]
    if closure == "screws-top":
        pilot_depth = sc["length_mm"] - lid_t + 1.5
        for p in sc["at_mm"]:
            base -= cyl(p, sc["pilot_mm"] / 2, H - pilot_depth, pilot_depth + 1)
    elif closure == "screws-back":
        # Up from underneath: a counterbore for the head, then a clearance
        # hole the whole height of the corner column. The thread is in the lid.
        cb = sc["counterbore_mm"]
        for p in sc["at_mm"]:
            base -= cyl(p, head_d / 2 + 0.4, -1, cb + 1)
            base -= cyl(p, sc["clearance_mm"] / 2, -1, H + 2)

    b = L.get("board")
    snap = bool(b) and b.get("mount") == "snap"
    if b:
        for h in b["holes_mm"]:
            base += cyl(h, b["pilot_mm"] / 2 + 2.2, floor - 0.5, b["standoff_mm"] + 0.5)
            if snap:
                # A peg up through the board's hole: it locates, nothing screws.
                base += cyl(h, b["hole_mm"] / 2 - 0.15, floor + b["standoff_mm"], b["thickness_mm"] + 1.0)
            else:
                base -= cyl(h, b["pilot_mm"] / 2, floor + 0.5, b["standoff_mm"])
        if snap:
            # Snap hooks on the board's side edges: a ledge it rests on, a beam
            # beside its edge, a lip over its top that it clicks under.
            zb = floor + b["standoff_mm"]
            zt = zb + b["thickness_mm"]
            for i, hk in enumerate(b.get("hooks", [])):
                (hx, hy), w = hk["at_mm"], hk["width_mm"]
                sx = -1 if hk["side"] == "left" else 1
                ledge = box((hx - sx * 0.75, hy), (1.5, w), floor - 0.5, b["standoff_mm"] + 0.5)
                beam = box((hx + sx * (0.3 + 0.7), hy), (1.4, w), floor - 0.5, zt + 1.4 - floor + 0.5)
                lip = Pos(hx - sx * 0.4, hy, zt + 0.05) * Box(1.4, w, 0.8, align=(Align.CENTER, Align.CENTER, Align.MIN))
                # A lead-in chamfer on the lip's top so the board pushes it aside.
                solids[f"snap-{i}"] = (ledge + beam + lip, "plastic", "base")

    for s in L["sockets"]:
        rib_h = min(s["height_mm"] * 0.6, 12)
        leads = None
        if s.get("leads"):
            # Which end, and how far across it, from where the leads start.
            (bx, by) = s["body"]["centre_mm"]
            f = [l["from_mm"] for l in s["leads"]]
            axis = s.get("axis", "x")
            k = 0 if axis == "x" else 1
            sign = 1 if f[0][k] > (bx, by)[k] else -1
            across = [p[1 - k] for p in f]
            leads = (axis, sign, max(across) - min(across) + 1.2)
        base += l_locators(s["body"], clr, s["locator_mm"], 10, floor - 0.5, rib_h + 0.5, leads)

    for w in c["wall_parts"]:
        (x, y, z), (nx, ny) = w["centre_mm"], w["normal"]
        axis = Plane(origin=(x - nx * (wall + 2), y - ny * (wall + 2), z), z_dir=(nx, ny, 0))
        base -= axis * Cylinder(w["diameter_mm"] / 2, wall + 4, align=(Align.CENTER, Align.CENTER, Align.MIN))
    for op in c.get("openings", []):
        base -= opening(op)
    solids["case-base"] = (base, "plastic", "base")

    # ── Gaskets ─────────────────────────────────────────────────────────────
    if seal:
        g_in = seal["lip_mm"] + seal["groove_mm"] * 0.1
        g_out = seal["lip_mm"] + seal["groove_mm"] * 0.9
        ring = offset(cavity, amount=g_out, kind=Kind.ARC) - offset(cavity, amount=g_in, kind=Kind.ARC)
        # Drawn at its FREE height, standing proud of the rim: the part as it
        # comes off the printer. In the assembled pose the lid overlaps it by
        # exactly the squeeze — that overlap is the seal, and checks() measures it.
        solids["gasket-lid"] = (slab(ring, H - seal["depth_mm"], seal["gasket_height_mm"]), "tpu", "base")

    # ── Lid ─────────────────────────────────────────────────────────────────
    lid = slab(outline, H, lid_t)
    if seal:
        t_in = seal["lip_mm"] + seal["groove_mm"] * 0.25
        t_out = seal["lip_mm"] + seal["groove_mm"] * 0.75
        tongue = offset(cavity, amount=t_out, kind=Kind.ARC) - offset(cavity, amount=t_in, kind=Kind.ARC)
        if seal["tongue_mm"] > 1e-6:
            lid += slab(tongue, H - seal["tongue_mm"], seal["tongue_mm"])
    if closure == "screws-top":
        for p in sc["at_mm"]:
            lid -= cyl(p, sc["clearance_mm"] / 2, H - 2, lid_t + 4)
            lid -= cyl(p, head_d / 2 + 0.4, H + lid_t - head_h - 0.3, head_h + 1)
    elif closure == "screws-back":
        # A blind pilot from underneath, stopping 1 mm short of the top face:
        # the face stays unbroken.
        for p in sc["at_mm"]:
            lid -= cyl(p, sc["pilot_mm"] / 2, H - 1, lid_t)
    else:
        # Press-fit: a skirt just inside the cavity wall, oversize by the
        # declared interference.
        pf = c["press_fit"]
        skirt = offset(cavity, amount=pf["interference_mm"], kind=Kind.ARC) - offset(
            cavity, amount=-1.6, kind=Kind.ARC
        )
        lid += slab(skirt, H - pf["skirt_mm"], pf["skirt_mm"])

    for rp in L["region_parts"]:
        body = rp["body"]
        (cx, cy), (w, d) = body["centre_mm"], body["size_mm"]
        t = rp["thickness_mm"]
        if rp["mount"] == "window":
            lid -= slab(face(rp["region_mm"]), H - 1, lid_t + 2)
            wg = seal["window_gasket_mm"] if seal else 0
            ov = rp["overlap_mm"]
            if seal:
                frame = offset(face(rp["region_mm"]), amount=ov - 0.5, kind=Kind.ARC) - offset(
                    face(rp["region_mm"]), amount=0.5, kind=Kind.ARC
                )
                solids[f"gasket-{rp['part']}"] = (slab(frame, H - wg, wg), "tpu", "lid")
            lid += l_locators(body, clr, 2.0, 10, H - wg - t, wg + t + 0.5)
            solids[rp["part"]] = (box((cx, cy), (w, d), H - wg - t, t), L["looks"].get(rp["part"], "panel"), "lid")
        elif rp["mount"] == "surface":
            # On top of the lid. The adhesive foam ring bonds it and seals the
            # hole its leads go down through; the hole is inside the ring.
            top = H + lid_t
            lid -= cyl((cx, cy), rp["pass_through_mm"] / 2, H - 1, lid_t + 2)
            aw, at_ = rp["adhesive_width_mm"], rp["adhesive_mm"]
            if rp.get("flush"):
                # Sunk into a pocket: it drops in, and its face is the lid's.
                top = H + lid_t - at_ - t
                lid -= box((cx, cy), (w + 0.6, d + 0.6), top, at_ + t + 1)
            ring = box((cx, cy), (w, d), top, at_) - box((cx, cy), (w - 2 * aw, d - 2 * aw), top - 1, at_ + 2)
            solids[f"adhesive-{rp['part']}"] = (ring, "adhesive", "top")
            z0 = top + at_
            panel = box((cx, cy), (w, d), z0, t)
            solids[rp["part"]] = (panel, L["looks"].get(rp["part"], "panel"), "top")
            # Cell strings and busbars on the face, so it reads as a panel.
            lines = None
            for k in range(1, 4):
                x = cx - w / 2 + k * w / 4
                seg = box((x, cy), (0.5, d - 4), z0 + t, 0.15)
                lines = seg if lines is None else lines + seg
            for k in range(1, 6):
                y = cy - d / 2 + k * d / 6
                seg = box((cx, y), (w - 4, 0.25), z0 + t, 0.1)
                lines = lines + seg
            solids[f"{rp['part']}-cells"] = (lines, "busbar", "top")
            # Its solder pins on its back, inside the ring, over the hole: where
            # the wire engine starts each core, so wire and pin meet.
            starts = [cr["path_mm"][0][:2] for wr in L.get("wires", []) if wr["from"] == rp["part"] for cr in wr.get("cores_mm", [])]
            starts = starts or [(cx - 2.0, cy), (cx + 2.0, cy)]
            pins = None
            for sp in starts:
                pin = cyl(sp, 0.6, top - 0.5, at_ + 0.5)
                pins = pin if pins is None else pins + pin
            solids[f"{rp['part']}-pins"] = (pins, "pin", "top")
        elif rp["mount"] == "vent":
            # A hole through the lid, and the membrane bonded over it inside:
            # air and vapour through, water not.
            lid -= cyl((cx, cy), rp["pass_through_mm"] / 2, H - 1, lid_t + 2)
            if rp.get("membrane") == "outside":
                # On the lid's face — at the floor of the mark it sits in,
                # when that mark is cut in, so it stays below the surface.
                mk = next((m for m in c.get("marks", []) if m.get("id") == rp["region"]), None)
                sunk = mk["depth_mm"] if mk and mk["kind"] not in ("emboss", "etch") else 0.0
                solids[rp["part"]] = (cyl((cx, cy), w / 2, H + lid_t - sunk, t), "membrane", "lid")
            else:
                solids[rp["part"]] = (cyl((cx, cy), w / 2, H - t, t), "membrane", "lid")
        elif rp["mount"] == "adhesive":
            # Bonded flat to the lid's underside.
            solids[rp["part"]] = (box((cx, cy), (w, d), H - t, t), L["looks"].get(rp["part"], "flex"), "lid")
        else:  # pocket in the top of the lid
            lid -= box((cx, cy), (w + 2 * clr, d + 2 * clr), H + lid_t - t, t + 1)
            solids[rp["part"]] = (box((cx, cy), (w, d), H + lid_t - t, t), L["looks"].get(rp["part"], "panel"), "lid")
    # Finish: break every top edge of the lid (the outline and the window
    # rims). Cosmetic, so a kernel that cannot chamfer some edge leaves it
    # sharp and says so rather than failing the build.
    try:
        top = lid.edges().filter_by_position(Axis.Z, H + lid_t - 1e-3, H + lid_t + 1e-3).filter_by(GeomType.LINE)
        lid = chamfer(top, 0.8)
    except Exception as e:  # noqa: BLE001 — cosmetic only
        print(f"note: lid edges left sharp ({type(e).__name__})", file=sys.stderr)
    # Marks on the lid's face: cut in, raised, or their outline etched.
    top_z = H + lid_t
    for m in c.get("marks", []):
        shape = face(m["polygon_mm"])
        dpt = m["depth_mm"]
        if m["kind"] == "emboss":
            lid += slab(shape, top_z, dpt)
        elif m["kind"] == "etch":
            lid -= slab(shape - offset(shape, amount=-m["line_mm"], kind=Kind.ARC), top_z - dpt, dpt + 1)
        else:
            lid -= slab(shape, top_z - dpt, dpt + 1)
    if hg and hg.get("kind") == "holes":
        # Through the solid tip: front to back through base and lid, and side
        # to side through the base. Solid plastic all round, outside the seal.
        th = hg["through"]
        hole = cyl(th["at_mm"], th["d_mm"] / 2, -1, H + lid_t + 2)
        base -= hole
        lid -= hole
        ac = hg["across"]
        span = ac["to_x_mm"] - ac["from_x_mm"]
        base -= Pos(ac["from_x_mm"], ac["y_mm"], H / 2) * Rot(0, 90, 0) * Cylinder(ac["d_mm"] / 2, span, align=(Align.CENTER, Align.CENTER, Align.MIN))
        solids["case-base"] = (base, "plastic", "base")
    for op in c.get("openings", []):
        if op["mouth_mm"][2] + op["size_mm"][1] / 2 > H:
            lid -= opening(op)
    solids["case-lid"] = (lid, "plastic-lid", "lid")

    # ── Screws ──────────────────────────────────────────────────────────────
    d_nom = float(sc["size"].lstrip("M")) if sc["at_mm"] else 0
    hexagon = lambda r: make_face(Polyline(*[(r * math.cos(math.pi * k / 3), r * math.sin(math.pi * k / 3)) for k in range(6)], close=True))
    for i, p in enumerate(sc["at_mm"]):
        if closure == "screws-back":
            # Head up in its counterbore, shank up through the base into the lid.
            head_top = sc["counterbore_mm"]
            s = cyl(p, head_d / 2, head_top - head_h, head_h) + cyl(p, d_nom / 2 * 0.95, head_top, sc["length_mm"])
            s -= Pos(p[0], p[1], head_top - head_h - 0.01) * extrude(hexagon(d_nom * 0.45), amount=head_h * 0.6)
        else:
            top = H + lid_t - 0.3
            s = cyl(p, head_d / 2, top - head_h, head_h) + cyl(p, d_nom / 2 * 0.95, top - head_h - sc["length_mm"], sc["length_mm"])
            s -= Pos(p[0], p[1], top - head_h * 0.6) * extrude(hexagon(d_nom * 0.45), amount=head_h)
        solids[f"screw-{i}"] = (s, "steel", "screws")

    # ── Board and what rides on it ──────────────────────────────────────────
    if b:
        (bx, by), (bw, bd) = b["body"]["centre_mm"], b["body"]["size_mm"]
        z = b["z_bottom_mm"]
        pcb = box((bx, by), (bw, bd), z, b["thickness_mm"])
        for h in b["holes_mm"]:
            pcb -= cyl(h, b["hole_mm"] / 2, z - 1, b["thickness_mm"] + 2)
        # Plated holes for leads soldered through the board.
        for q in b["illustrative_placement"]:
            if q.get("drill_mm"):
                for pd in q.get("pads_at") or []:
                    pcb -= cyl(pd["at_mm"], q["drill_mm"] / 2, z - 1, b["thickness_mm"] + 2)
            # And every hole a footprint drills: a connector's shell legs and
            # locating pegs go into the board, which is not a collision.
            for pd in q.get("pads_at") or []:
                if dr := pd.get("drill_mm"):
                    w, h = dr
                    if abs(w - h) < 1e-6:
                        pcb -= cyl(pd["at_mm"], w / 2, z - 1, b["thickness_mm"] + 2)
                    else:  # an oval drill, as a slot: a box and its two round ends
                        r = min(w, h) / 2
                        (px, py), along_x = pd["at_mm"], w > h
                        d = (max(w, h) - min(w, h)) / 2
                        ends = [(px - d, py), (px + d, py)] if along_x else [(px, py - d), (px, py + d)]
                        pcb -= box((px, py), (max(w, h) - 2 * r if along_x else w, h - 2 * r if not along_x else h), z - 1, b["thickness_mm"] + 2)
                        for e in ends:
                            pcb -= cyl(e, r, z - 1, b["thickness_mm"] + 2)
        solids["board"] = (pcb, "pcb", "board")
        top = z + b["thickness_mm"]
        routed = (BUILD / "copper.json").exists()
        for q in b["illustrative_placement"]:
            nm = q["part"] if q["n"] == 0 else f"{q['part']}-{q['n']}"
            board_part(solids, nm, q, top, L["looks"].get(q["part"], "component"), routed)
            if q.get("keepout"):
                # Drawn as a film on the board: a part that intrudes on an RF
                # keep-out then fails the interference check by name.
                solids[f"{nm}-keepout"] = (box(q["keepout"]["centre_mm"], q["keepout"]["size_mm"], top, 0.02), "keepout", "board")
        # A vent's chimney: a tube printed with the lid, from round the
        # membrane down to a soft ring squeezed on the board round its part.
        for ch in L.get("chimneys", []):
            at, top, r_in = ch["centre_mm"], ch.get("top_mm", ch["centre_mm"]), ch["bore_mm"] / 2
            r_out = r_in + ch["wall_mm"]
            z0 = ch["tube_bottom_mm"]
            # From round its part on the board up to round the vent: it leans
            # when the part could not sit straight under the vent.
            bore = lean(at, top, r_in, z0, H)
            bore += cyl(at, r_in, z0 - 1, 1.01) + cyl(top, r_in, H - 0.01, 1)
            solids[f"chimney-{ch['vent']}"] = (lean(at, top, r_out, z0, H) - bore, "plastic-lid", "lid")
            g = ch["gasket"]
            ring = cyl(at, g["outer_mm"] / 2, ch["board_top_mm"], g["squeezed_mm"]) - cyl(at, g["inner_mm"] / 2, ch["board_top_mm"] - 1, g["squeezed_mm"] + 2)
            solids[f"chimney-ring-{ch['vent']}"] = (ring, "tpu", "lid")
        for i, h in enumerate(b["holes_mm"] if not snap else []):
            hd = b["hole_mm"] * 1.8
            solids[f"board-screw-{i}"] = (cyl(h, hd / 2, top, 1.8) + cyl(h, b["hole_mm"] / 2 * 0.85, top - b["thickness_mm"] - 3.5, b["thickness_mm"] + 3.5), "steel", "board")

    # ── Socketed parts ──────────────────────────────────────────────────────
    for s in L["sockets"]:
        (x, y), (w, d) = s["body"]["centre_mm"], s["body"]["size_mm"]
        nm = s["part"] if s.get("n", 0) == 0 else f"{s['part']}-{s['n']}"
        if s["shape"] == "cylinder":
            along_y = s.get("axis") == "y"
            length, dia = (d, w) if along_y else (w, d)
            r = min(dia, s["height_mm"]) / 2
            turn = Rot(90, 0, 0) if along_y else Rot(0, 90, 0)
            holder = box((x, y), (w, d), floor, s["height_mm"] * 0.45) - (Pos(x, y, floor + r) * turn * Cylinder(r + 0.1, length + 1))
            cell = Pos(x, y, floor + r) * turn * Cylinder(r - 0.3, length - 0.4)
            solids[f"{nm}-holder"] = (holder, "holder", "socket")
            solids[nm] = (cell, L["looks"].get(s["part"], "cell"), "socket")
            # Its leads, out of the end that faces the board, to where the
            # wires are soldered on.
            for k, lead in enumerate(s.get("leads", [])):
                # Out of the can, straight to its hole, and — when it lands in
                # the board — bent down through it.
                pts = [tuple(lead["from_mm"]), tuple(lead["at_mm"])]
                if lead.get("hole_mm") and b:
                    hx, hy = lead["hole_mm"]
                    pts.append((hx, hy, b["z_bottom_mm"] - 0.8))
                rod = None
                for p0, p1 in zip(pts, pts[1:]):
                    v = (p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2])
                    ln = math.sqrt(sum(c * c for c in v))
                    if ln < 1e-6:
                        continue
                    seg = Plane(origin=p0, z_dir=v) * Cylinder(0.3, ln, align=(Align.CENTER, Align.CENTER, Align.MIN))
                    rod = seg if rod is None else rod + seg + Pos(*p0) * Sphere(0.3)
                solids[f"{nm}-lead-{lead['pin'].replace('+', 'plus').replace('-', 'minus')}"] = (rod, "busbar", "wires")
        else:
            solids[nm] = (box((x, y), (w, d), floor, s["height_mm"]), L["looks"].get(s["part"], "cell"), "socket")

    # ── Wires ───────────────────────────────────────────────────────────────
    # Drawn along the solved route, two cores side by side. Drawn, not
    # interference-checked: a real wire bends round what it meets.
    # A single-core wire takes the colour of the net it lands on: ground black,
    # a supply red, anything else yellow.
    nets = {}
    for q in (b or {}).get("illustrative_placement", []):
        for pad, net in (q.get("pin_nets") or {}).items():
            nets[f"{q['part']}.{pad}"] = net

    def colour(wr):
        net = nets.get(wr["to"]) or nets.get(wr["from"]) or ""
        if net == "GND" or net.endswith("-"):
            return "wire-black"
        return "wire-red" if net.endswith("+") or net in ("3V3", "VCC", "VDD") else "wire-yellow"

    def core_look(net, core):
        if net:
            return "wire-black" if net == "GND" or net.endswith("-") else ("wire-red" if net.endswith("+") or net in ("3V3", "VCC", "VDD") else "wire-yellow")
        return "wire-red" if core == 0 else "wire-black"

    for wr in L.get("wires", []):
        path = [tuple(q) for q in wr["path_mm"]]
        if wr.get("cores", 2) == 1:
            wr["look"] = colour(wr)
        coax = wr.get("kind") == "coax"
        if coax:
            wr["look"] = "coax"
        radius = 0.57 if coax else 0.6
        # Each core along its own solved path (fid-hardware's wire engine):
        # parallel, never crossing, each onto its own pad.
        solved = wr.get("cores_mm") or []
        for core in range(len(solved) or wr.get("cores", 2)):
            if solved:
                pts = [tuple(q) for q in solved[core]["path_mm"]]
            else:
                off = (core - (wr.get("cores", 2) - 1) / 2) * 1.3
                pts = [(q[0] + off * 0.7071, q[1] + off * 0.7071, q[2]) for q in path]
            tube = None
            for a, bq in zip(pts, pts[1:]):
                v = (bq[0] - a[0], bq[1] - a[1], bq[2] - a[2])
                ln = math.sqrt(sum(t * t for t in v))
                if ln < 1e-6:
                    continue
                seg = Plane(origin=a, z_dir=v) * Cylinder(radius, ln, align=(Align.CENTER, Align.CENTER, Align.MIN))
                tube = seg if tube is None else tube + seg
            for q in pts[1:-1]:
                tube = tube + Pos(*q) * Sphere(radius)
            look = "coax" if coax else (core_look(solved[core]["net"], core) if solved else (wr.get("look") or ("wire-red" if core == 0 else "wire-black")))
            solids[f"wire-{wr['id']}-{core}"] = (tube, look, "wires")

    # ── Wall parts ──────────────────────────────────────────────────────────
    for wp in c["wall_parts"]:
        (x, y, z), (nx, ny) = wp["centre_mm"], wp["normal"]
        r = wp["diameter_mm"] / 2 - clr
        out_pl = Plane(origin=(x, y, z), z_dir=(nx, ny, 0))
        in_pl = Plane(origin=(x - nx * wall, y - ny * wall, z), z_dir=(-nx, -ny, 0))
        thread = Plane(origin=(x - nx * (wall + wp["inside_mm"] * 0.6), y - ny * (wall + wp["inside_mm"] * 0.6), z), z_dir=(nx, ny, 0)) * Cylinder(
            r, wall + wp["inside_mm"] * 0.6 + 2, align=(Align.CENTER, Align.CENTER, Align.MIN)
        )
        nut = in_pl * extrude(make_face(Polyline(*[(r * 1.7 * math.cos(math.pi * k / 3), r * 1.7 * math.sin(math.pi * k / 3)) for k in range(6)], close=True)), amount=min(3, wp["inside_mm"] * 0.4))
        cap = out_pl * Cylinder(r * 1.6, 2.5, align=(Align.CENTER, Align.CENTER, Align.MIN))
        stem = (Plane(origin=(x + nx * 2.5, y + ny * 2.5, z), z_dir=(nx, ny, 0)) * Cylinder(r * 0.9, wp["outside_mm"] - 2.5, align=(Align.CENTER, Align.CENTER, Align.MIN)))
        solids[wp["part"]] = (thread + nut + cap + stem, L["looks"].get(wp["part"], "metal"), "wall")

    return solids


def checks(L, solids):
    c = L["case"]
    H = c["base_height_mm"]
    out = {"interference": [], "insertion": [], "lid": [], "screws": [], "seal": [], "models": []}
    # Overlaps a design intends: a screw is meant to cut its own thread; a
    # press-fit lid is meant to overlap the base by its interference, and the
    # lid is meant to squeeze the gasket — both measured on their own below.
    # Wires are drawn, not checked.
    press = c.get("press_fit")
    # A part sits on its own copper.
    intended = lambda a, b: (
        any(x.startswith(("screw-", "board-screw-", "wire-")) or "-lead-" in x for x in (a, b))
        or any(x.startswith(y + "-copper") for x, y in ((a, b), (b, a)))
        or (press is not None and {a, b} == {"case-base", "case-lid"})
        or {a, b} == {"gasket-lid", "case-lid"}
    )
    # The seal: the lid must press into the gasket — by at least its
    # protrusion everywhere, and no further than the declared compression.
    # A gasket the lid does not reach seals nothing, whatever the picture shows.
    seal = c.get("seal")
    if seal and "gasket-lid" in solids:
        g = solids["gasket-lid"][0]
        area = g.volume / seal["gasket_height_mm"]
        squeezed = (g & solids["case-lid"][0]).volume
        # On average at least half the declared squeeze: a flush gasket (0 proud,
        # no tongue) must fail here, not pass on a lower bound of zero.
        least = area * seal["gasket_height_mm"] * seal["compression"] * 0.5
        most = area * seal["gasket_height_mm"] * seal["compression"] * 1.1
        if squeezed < least:
            out["seal"].append({
                "problem": "the lid does not squeeze the gasket — it seals nothing",
                "squeezed_mm3": round(squeezed, 3), "needs_at_least_mm3": round(least, 3),
            })
        elif squeezed > most:
            out["seal"].append({
                "problem": "the gasket is crushed beyond its declared compression",
                "squeezed_mm3": round(squeezed, 3), "at_most_mm3": round(most, 3),
            })
    # A chimney seals one part in with the outside air, and only that one:
    # anything else inside its bore is wet, and its ring would not seal.
    for ch in L.get("chimneys", []):
        (cx, cy), r = ch["centre_mm"], ch["ring_mm"] / 2
        if ch["tube_bottom_mm"] >= H - 0.5:
            out["seal"].append({"chimney": ch["vent"], "problem": "no room for the tube between board and lid"})
        for q in (L.get("board") or {}).get("illustrative_placement", []):
            (px, py), (pw, ph) = q["body"]["centre_mm"], q["body"]["size_mm"]
            dx = max(abs(px - cx) - pw / 2, 0.0)
            dy = max(abs(py - cy) - ph / 2, 0.0)
            if q["part"] != ch["part"] and dx * dx + dy * dy < r * r:
                out["seal"].append({"chimney": ch["vent"], "problem": f"`{q['part']}` is inside the chimney's ring — only `{ch['part']}` may be"})
            if q["part"] == ch["part"] and (abs(px - cx) + pw / 2) ** 2 + (abs(py - cy) + ph / 2) ** 2 > (ch["bore_mm"] / 2) ** 2:
                out["seal"].append({"chimney": ch["vent"], "problem": f"`{q['part']}` is not wholly inside the chimney's bore"})
        # Wires are not solids here, so they are checked against the tube by
        # distance: none may pass through it.
        tube = ch["bore_mm"] / 2 + ch["wall_mm"]
        top = ch.get("top_mm", ch["centre_mm"])
        axis = [(cx + (top[0] - cx) * f, cy + (top[1] - cy) * f) for f in (0.0, 0.5, 1.0)]
        lean_by = math.hypot(top[0] - cx, top[1] - cy)
        if lean_by > 0.6 * (H - ch["tube_bottom_mm"]) + 1e-6:
            out["seal"].append({"chimney": ch["vent"], "problem": f"the tube leans {lean_by:.1f} mm over {H - ch['tube_bottom_mm']:.1f} mm — too steep to print unsupported"})
        for wr in L.get("wires", []):
            for core in wr.get("cores_mm", []):
                pts = core["path_mm"]
                hit = False
                for a, bb in zip(pts, pts[1:]):
                    dx, dy = bb[0] - a[0], bb[1] - a[1]
                    for ax, ay in axis:
                        t = max(0.0, min(1.0, ((ax - a[0]) * dx + (ay - a[1]) * dy) / (dx * dx + dy * dy or 1e-9)))
                        hit = hit or math.hypot(a[0] + t * dx - ax, a[1] + t * dy - ay) < tube
                if hit:
                    out["seal"].append({"chimney": ch["vent"], "problem": f"the wire {wr['from']} → {wr['to']} passes through the chimney"})
    names = sorted(solids)
    for i, a in enumerate(names):
        for bn in names[i + 1:]:
            if intended(a, bn):
                continue
            sa, sb = solids[a][0], solids[bn][0]
            if not sa.bounding_box().overlaps(sb.bounding_box()):
                continue
            v = (sa & sb).volume
            if v > TOL:
                out["interference"].append({"a": a, "b": bn, "mm3": round(v, 3)})
    # A socket reserves its *declared* envelope, not the shape drawn in it: a
    # cell drawn as a cylinder must not let the declared height go unchecked.
    floor = c["floor_mm"]
    for s in L["sockets"]:
        env = box(s["body"]["centre_mm"], s["body"]["size_mm"], floor, s["height_mm"])
        for other, (so, _, g) in solids.items():
            if g in ("lid", "board", "wall") and not other.startswith("board-screw"):
                v = (env & so).volume
                if v > TOL:
                    out["interference"].append({"a": f"{s['part']} (declared envelope)", "b": other, "mm3": round(v, 3)})
    base = solids["case-base"][0]
    for nm, (s, _, group) in solids.items():
        if group not in ("socket", "board") or nm.startswith("board-screw"):
            continue
        bb = s.bounding_box()
        sweep = Pos(0, 0, bb.max.Z) * Box(bb.size.X - 0.02, bb.size.Y - 0.02, H + 20 - bb.max.Z, align=(Align.MIN, Align.MIN, Align.MIN))
        sweep = Pos(bb.min.X + 0.01, bb.min.Y + 0.01, 0) * sweep
        brd = L.get("board")
        if nm == "board" and brd and brd.get("mount") == "snap":
            # Its holes go down over the locating pegs: they are meant to.
            for h in brd["holes_mm"]:
                sweep -= cyl(h, brd["hole_mm"] / 2, 0, H + 40)
        v = (sweep & base).volume
        if v > TOL:
            out["insertion"].append({"part": nm, "blocked_mm3": round(v, 3)})
    lid_group = [n for n, (_, _, g) in solids.items() if g == "lid"]
    below = [n for n, (_, _, g) in solids.items() if g in ("socket", "board", "wall")]
    for ln in lid_group:
        for bn in below:
            v = (solids[ln][0] & solids[bn][0]).volume
            if v > TOL:
                out["lid"].append({"lid_part": ln, "hits": bn, "mm3": round(v, 3)})
    sc = c["screws"]
    closure = c.get("closure", "screws-top")
    if closure != "press-fit":
        need = 1.5 * float(sc["size"].lstrip("M"))
        if closure == "screws-back":
            engage = sc["length_mm"] - (H - sc["counterbore_mm"])
            room = c["lid_mm"] - 1.0
            if engage > room + 1e-6:
                out["screws"].append({"engagement_mm": round(engage, 3), "lid_allows_mm": room, "problem": "screw would break through the lid face"})
        else:
            engage = sc["length_mm"] - c["lid_mm"]
        if engage < need - 1e-6:
            out["screws"].append({"engagement_mm": round(engage, 3), "needs_mm": need})
    else:
        # The skirt should overlap the base by about its interference band, no more.
        ov = (solids["case-lid"][0] & solids["case-base"][0]).volume
        expected = fit_perimeter(c["outline_mm"]) * press["interference_mm"] * press["skirt_mm"]
        if ov > 3 * expected + 1:
            out["lid"].append({"lid_part": "case-lid", "hits": "case-base", "mm3": round(ov, 3), "expected_press_fit_mm3": round(expected, 3)})
    return out


def fit_perimeter(pts):
    return sum(math.dist(pts[i], pts[(i + 1) % len(pts)]) for i in range(len(pts)))


EXPLODE = {"base": 0, "socket": 25, "board": 45, "wall": 0, "lid": 110, "top": 145, "screws": 150, "wires": 0}

LOOK = {  # colour, metalness, roughness, opacity — the renderer reads these
    # The case: `case.colour` when declared (see main), else a neutral grey.
    "plastic": ["#9a9c9f", 0.0, 0.55, 1.0],
    "plastic-lid": ["#aeb0b3", 0.0, 0.5, 1.0],
    "tpu": ["#1b1b1f", 0.0, 0.9, 1.0],
    "panel": ["#1c2a4a", 0.3, 0.2, 1.0],
    "pcb": ["#0f4a28", 0.0, 0.5, 1.0],
    "shield": ["#c9ccd1", 0.9, 0.3, 1.0],
    "component": ["#26262b", 0.1, 0.5, 1.0],
    "holder": ["#141416", 0.0, 0.7, 1.0],
    "cell": ["#2f7dd6", 0.2, 0.35, 1.0],
    "metal": ["#b8a36a", 0.9, 0.35, 1.0],
    "steel": ["#9aa0a6", 0.95, 0.3, 1.0],
    "adhesive": ["#4a4e57", 0.0, 0.85, 1.0],
    "busbar": ["#d6d9de", 0.9, 0.25, 1.0],
    "pin": ["#d4af37", 0.95, 0.25, 1.0],
    "wire-red": ["#c62828", 0.0, 0.45, 1.0],
    "wire-black": ["#16161a", 0.0, 0.45, 1.0],
    "wire-yellow": ["#e0b020", 0.0, 0.45, 1.0],
    "coax": ["#5c5f66", 0.1, 0.5, 1.0],
    "membrane": ["#f2f1ec", 0.0, 0.9, 1.0],
    "flex": ["#b8732a", 0.2, 0.45, 1.0],
    "keepout": ["#e0433a", 0.0, 0.9, 0.35],
}


def lighter(hex_colour: str, by: float) -> str:
    """The same colour, `by` of the way to white: the lid, a shade off the base."""
    rgb = [int(hex_colour[k:k + 2], 16) for k in (1, 3, 5)]
    return "#" + "".join(f"{round(v + (255 - v) * by):02x}" for v in rgb)


def main() -> int:
    L = json.loads(LAYOUT.read_text())
    if colour := L["case"].get("colour"):
        LOOK["plastic"][0] = colour
        LOOK["plastic-lid"][0] = lighter(colour, 0.12)
    BUILD.mkdir(parents=True, exist_ok=True)
    # A part that is no longer declared must not linger as a solid from an
    # earlier build: the viewer and a reviewer would both believe it.
    for old in list(BUILD.glob("*.stl")) + list(BUILD.glob("*.step")):
        old.unlink()
    solids = build(L)
    model_problems = {"models": []}
    apply_models(L, solids, model_problems)
    custom = product_shapes(L, solids)
    result = checks(L, solids)
    result["models"] += model_problems["models"]
    for nm, (s, _, _) in solids.items():
        if not s.is_valid:
            result.setdefault("validity", []).append(nm)
    ok = not any(result.values())
    result["ok"] = ok
    if custom is not None:
        # Not a check — a record, so a reviewer knows which solids are the product's own.
        result["custom"] = custom
        print(f"hardware/shapes.py: added {len(custom['added'])}, changed {len(custom['changed'])}, removed {len(custom['removed'])} solid(s)")

    scene = {"product": L["product"]["name"], "why": L["why"], "parts": [], "looks": LOOK}
    for nm, (s, look, group) in sorted(solids.items()):
        export_step(s, BUILD / f"{nm}.step")
        explode = EXPLODE[group]
        if group == "screws" and L["case"].get("closure") == "screws-back":
            explode = -45  # they come out of the underside
        # A part with a coloured model renders as its colours; everything
        # else as one mesh in its look.
        for i, (piece, colour) in enumerate(RENDERS.get(nm, [(s, None)])):
            stl = f"{nm}.stl" if colour is None else f"{nm}~{i}.stl"
            export_stl(piece, BUILD / stl, tolerance=0.01, angular_tolerance=0.15)
            key = look if colour is None else colour
            if colour is not None and colour not in LOOK:
                # Bright, unsaturated faces are metal (leads, shields, tin).
                r, g, bl = (int(colour[k:k + 2], 16) / 255 for k in (1, 3, 5))
                metal = max(r, g, bl) > 0.55 and max(r, g, bl) - min(r, g, bl) < 0.35
                LOOK[colour] = [colour, 0.85 if metal else 0.05, 0.3 if metal else 0.55, 1.0]
            scene["parts"].append({"name": nm, "stl": stl, "look": key, "group": group, "explode_mm": explode})
    # The routed board's copper, drawn by the viewer straight from KiCad's
    # geometry; with the transform from KiCad's coordinates to the case's.
    if b := L.get("board"):
        copper = BUILD / "copper.json"
        if copper.exists():
            (bx, by), (bw, bh) = b["body"]["centre_mm"], b["body"]["size_mm"]
            scene["copper"] = {
                **json.loads(copper.read_text()),
                # x = xk - 100 + ox; y = 100 + h + oy - yk (see render_kicad)
                "origin": [bx - bw / 2 - 100.0, by - bh / 2 + 100.0 + bh],
                "z_top": b["z_bottom_mm"] + b["thickness_mm"],
                "z_bottom": b["z_bottom_mm"],
                "explode_mm": EXPLODE["board"],
            }
    scene["legend"] = sorted({p["look"] for p in scene["parts"] if not p["look"].startswith("#")})
    export_step(Compound([s for s, _, _ in solids.values()]), BUILD / "assembly.step")
    (BUILD / "scene.json").write_text(json.dumps(scene, indent=2) + "\n")
    (BUILD / "checks.json").write_text(json.dumps(result, indent=2) + "\n")

    bb = solids["case-base"][0].bounding_box()
    print(f"{len(solids)} solids; case {bb.size.X:.1f} × {bb.size.Y:.1f} × {bb.size.Z + L['case']['lid_mm']:.1f} mm")
    for k in ("interference", "insertion", "lid", "screws", "seal", "models", "validity"):
        for item in result.get(k, []):
            print(f"FAIL {k}: {item}", file=sys.stderr)
    print("checks: " + ("all pass" if ok else "FAILED — see hardware/build/checks.json"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
