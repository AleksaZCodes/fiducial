#!/usr/bin/env python3
"""The board as placed, with its ratsnest: hardware/build/review/ratsnest.svg.

    python3 hardware/ratsnest.py

Platform-owned (installed by `fid add capability hardware`). No dependencies.

Reads hardware/generated/layout.json — every part's courtyard and every pad
with its net — and draws each net as the shortest tree joining its pads, the
straight lines a router starts from. GND is left out: it is a pour.

Look at this before routing. A line that crosses a part is a track that has
to go round it; a knot of crossings is where the router runs out of room.
`fid derive` places parts against this ratsnest; this is how a person checks
what it did.
"""

from __future__ import annotations

import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LAYOUT = ROOT / "hardware" / "generated" / "layout.json"
OUT = ROOT / "hardware" / "build" / "review" / "ratsnest.svg"
PALETTE = ["#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd", "#8c564b", "#e377c2", "#7f7f7f", "#bcbd22", "#17becf"]


def tree(points: list[tuple[float, float]]) -> list[tuple[tuple[float, float], tuple[float, float]]]:
    """Prim's minimum spanning tree: the shortest set of lines joining every pad."""
    done, rest, edges = [points[0]], points[1:], []
    while rest:
        a, b = min(((a, b) for a in done for b in rest), key=lambda e: math.dist(*e))
        edges.append((a, b))
        done.append(b)
        rest.remove(b)
    return edges


def main() -> int:
    layout = json.loads(LAYOUT.read_text())
    board = layout["board"]
    (cx, cy), (w, h) = board["body"]["centre_mm"], board["body"]["size_mm"]
    x0, y1 = cx - w / 2, cy + h / 2
    k = 16.0  # px per mm
    X = lambda x: (x - x0 + 2) * k  # noqa: E731
    Y = lambda y: (y1 - y + 2) * k  # noqa: E731 — SVG's y points down
    svg = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{(w + 4) * k:.0f}" height="{(h + 4) * k:.0f}" font-family="sans-serif">',
        '<rect width="100%" height="100%" fill="#fff"/>',
        # The board's outline: its rectangle, or cut to the case (board.cut_mm).
        (f'<polygon points="{" ".join(f"{X(x):.1f},{Y(y):.1f}" for x, y in board["outline_mm"])}" '
         'fill="#e8f0e8" stroke="#000" stroke-width="2"/>') if board.get("outline_mm") else
        f'<rect x="{X(x0):.1f}" y="{Y(y1):.1f}" width="{w * k:.1f}" height="{h * k:.1f}" fill="#e8f0e8" stroke="#000" stroke-width="2"/>',
    ]
    nets: dict[str, list[tuple[float, float]]] = {}
    for p in board.get("illustrative_placement", []):
        (px, py), (pw, ph) = p["body"]["centre_mm"], p["body"]["size_mm"]
        svg.append(
            f'<rect x="{X(px - pw / 2):.1f}" y="{Y(py + ph / 2):.1f}" width="{pw * k:.1f}" height="{ph * k:.1f}" '
            'fill="none" stroke="#c0c" stroke-width="1"/>'
        )
        svg.append(f'<text x="{X(px):.1f}" y="{Y(py):.1f}" font-size="11" text-anchor="middle" fill="#035">{p.get("ref", p["part"])}</text>')
        pads = p.get("pads_at") or (p.get("footprint") or {}).get("pads") or []
        for q in pads:
            x, y = q["at_mm"]
            svg.append(f'<rect x="{X(x) - 2.5:.1f}" y="{Y(y) - 2.5:.1f}" width="5" height="5" fill="#c33"/>')
            net = (p.get("pin_nets") or {}).get(q["pin"])
            if net and net != "GND":
                nets.setdefault(net, []).append((x, y))
    for i, (net, pts) in enumerate(sorted(nets.items())):
        colour = PALETTE[i % len(PALETTE)]
        for a, b in tree(pts):
            svg.append(
                f'<line x1="{X(a[0]):.1f}" y1="{Y(a[1]):.1f}" x2="{X(b[0]):.1f}" y2="{Y(b[1]):.1f}" '
                f'stroke="{colour}" stroke-width="1.5"><title>{net}</title></line>'
            )
    for ch in layout.get("chimneys", []):
        # The ring a vent's tube seals on: nothing but its part inside it.
        (cx, cy), r = ch["centre_mm"], ch["ring_mm"] / 2
        svg.append(f'<circle cx="{X(cx):.1f}" cy="{Y(cy):.1f}" r="{r * k:.1f}" fill="none" stroke="#08c" stroke-width="2" stroke-dasharray="6 4"/>')
    for hole in board.get("holes_mm", []):
        svg.append(f'<circle cx="{X(hole[0]):.1f}" cy="{Y(hole[1]):.1f}" r="{1.5 * k:.1f}" fill="none" stroke="#080" stroke-width="2"/>')
    svg.append(f'<text x="8" y="16" font-size="13">ratsnest — GND omitted (a pour); {len(nets)} nets</text>')
    svg.append("</svg>")
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(svg) + "\n")
    print(f"wrote {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
