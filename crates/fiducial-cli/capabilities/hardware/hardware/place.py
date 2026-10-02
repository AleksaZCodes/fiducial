#!/usr/bin/env python3
"""Where every board part goes and which way it turns, by constraint programming.

    python3 hardware/place.py < hardware/generated/placement-model.json

Platform-owned (installed by `fid add capability hardware`). Needs `ortools`
(hardware/requirements.txt). `fid derive` runs it — from its own embedded
copy — whenever the placement model changes, and commits the answer to
hardware/generated/placement.json beside the model; run it by hand to see
what the solver makes of a model you are editing.

The model is the problem in fiducial's terms (spec 2026-10-01): items, each
with a size and pads per allowed rotation and a region it must stay inside;
fixed obstacles; and constraints of a few generic kinds. Every product rule
in product.toml is one instance of a kind; no rule has code of its own here.

  hard  inside     an item stays inside its region
        no_overlap items and obstacles stay `gap_mm` apart
        fixed      an item's centre (or one coordinate of it) is given
        flush      an item touches one side of its region (a connector at
                   the board edge it faces)
        within     an item's centre stays within `mm` of a point (a part
                   under a vent's leaning chimney)
        apart      two items stay at least `mm` apart, edge to edge
  cost  near       an item (or one of its pads) as close as it can get to a
                   point or to another item's pad, weighted
        net        a net's pads as close together as they can get: the
                   half-perimeter of their bounding box, weighted
        rotation   each turn but the first listed costs a little: ties go
                   to the declared orientation

Allowed rotations are the item's list: one entry is a fixed turn.

Deterministic: one worker, a fixed seed and deterministic budgets
(`effort`), in a large neighbourhood search of our own — so the same model
gives the same answer on the same ortools, on any machine.

Writes the answer to stdout as JSON. fid also sends the previous answer
as a hint: the search starts from it, so a small edit moves little. When the hard constraints cannot all
hold, the answer is `"status": "infeasible"` and `conflict` names a set of
them that cannot hold together — each by the declaration it came from.
"""

from __future__ import annotations

import json
import math
import sys

from ortools.sat.python import cp_model


def build(m: dict, explain: bool):
    g = m["grid_mm"]
    hg = m["gap_mm"] / 2.0
    # The grid starts at the board's corner. Bounds and sizes round to the
    # nearest step — within half a step, far inside any clearance — so a
    # zone sized exactly for its part still holds it.
    ox, oy = m["board"]["lo_mm"]

    def units(v: float) -> int:
        return round(v / g)

    md = cp_model.CpModel()
    guards: list[tuple[cp_model.IntVar, str]] = []

    def guard(why: str):
        """A hard constraint's switch: on, unless we are asking which ones conflict."""
        lit = md.NewBoolVar(why)
        guards.append((lit, why))
        if not explain:
            md.Add(lit == 1)
        return lit

    (bx0, by0), (bx1, by1) = (0.0, 0.0), (m["board"]["hi_mm"][0] - ox, m["board"]["hi_mm"][1] - oy)
    # The board, and any region reaching past it (a socket through the wall).
    regs = [it["region"] for it in m["items"] if it.get("region")]
    lo_x = min([bx0] + [r["lo_mm"][0] - ox for r in regs])
    hi_x = max([bx1] + [r["hi_mm"][0] - ox for r in regs])
    lo_y = min([by0] + [r["lo_mm"][1] - oy for r in regs])
    hi_y = max([by1] + [r["hi_mm"][1] - oy for r in regs])
    span = (units(lo_x) - 1, units(hi_x) + 1, units(lo_y) - 1, units(hi_y) + 1)
    # Bound on any doubled coordinate or distance, pads past the board included.
    big = 8 * max(abs(v) for v in span) + 1000
    xs, ys = [], []
    items: dict[str, dict] = {}
    costs: list[tuple[int, object]] = []

    for it in m["items"]:
        iid, rots = it["id"], it["rotations"]
        b = [md.NewBoolVar(f"{iid}@{r['deg']}") for r in rots]
        md.AddExactlyOne(b)
        # The size its rotation gives it: a variable, as an interval needs.
        w = md.NewIntVar(0, span[1] - span[0], f"{iid}.w")
        h = md.NewIntVar(0, span[3] - span[2], f"{iid}.h")
        md.Add(w == sum(bi * units(r["size_mm"][0]) for bi, r in zip(b, rots)))
        md.Add(h == sum(bi * units(r["size_mm"][1]) for bi, r in zip(b, rots)))
        x = md.NewIntVar(span[0], span[1], f"{iid}.x")
        y = md.NewIntVar(span[2], span[3], f"{iid}.y")
        # Twice the centre, in grid units: integer whatever the size's parity.
        cx2, cy2 = 2 * x + w, 2 * y + h
        reg = it.get("region")
        if reg:
            ok = guard(reg["from"])
            lx, ly = units(reg["lo_mm"][0] - ox), units(reg["lo_mm"][1] - oy)
            hx, hy = units(reg["hi_mm"][0] - ox), units(reg["hi_mm"][1] - oy)
            md.Add(x >= lx).OnlyEnforceIf(ok)
            md.Add(y >= ly).OnlyEnforceIf(ok)
            md.Add(x + w <= hx).OnlyEnforceIf(ok)
            md.Add(y + h <= hy).OnlyEnforceIf(ok)
            fl = it.get("flush")
            if fl:
                lit = guard(fl["from"])
                side = {"top": y + h == hy, "bottom": y == ly, "left": x == lx, "right": x + w == hx}[fl["side"]]
                md.Add(side).OnlyEnforceIf(lit)
        for f in it.get("fixed", []):
            lit = guard(f["from"])
            # Within one grid step: a size need not be a whole number of steps.
            for k, c2, o in (("x_mm", cx2, ox), ("y_mm", cy2, oy)):
                if k in f:
                    md.Add(c2 >= units(2 * (f[k] - o)) - 1).OnlyEnforceIf(lit)
                    md.Add(c2 <= units(2 * (f[k] - o)) + 1).OnlyEnforceIf(lit)
        # Within a distance of a point: an octagon round it, inside the
        # circle of that radius, so the bound holds in every direction.
        for wn in it.get("within", []):
            lit = guard(wn["from"])
            px, py = units(2 * (wn["at_mm"][0] - ox)), units(2 * (wn["at_mm"][1] - oy))
            r = int(2 * wn["mm"] / g / 1.0824)
            dx = md.NewIntVar(0, big, "")
            dy = md.NewIntVar(0, big, "")
            md.AddAbsEquality(dx, cx2 - px)
            md.AddAbsEquality(dy, cy2 - py)
            md.Add(dx <= r).OnlyEnforceIf(lit)
            md.Add(dy <= r).OnlyEnforceIf(lit)
            md.Add(dx + dy <= int(r * 1.4142)).OnlyEnforceIf(lit)
        # What it keeps clear besides its body (a keep-out to the board
        # edge), and half the gap: two items' halves make the whole gap.
        el, er, eb, et = it.get("keepout_mm", [0, 0, 0, 0])
        l, r_, bo, t = units(hg + el), units(hg + er), units(hg + eb), units(hg + et)
        x_end = md.NewIntVar(span[0] - 1000, span[1] + 1000, f"{iid}.x_end")
        y_end = md.NewIntVar(span[2] - 1000, span[3] + 1000, f"{iid}.y_end")
        md.Add(x_end == x + w + r_)
        md.Add(y_end == y + h + t)
        xs.append(md.NewIntervalVar(x - l, w + l + r_, x_end, f"{iid}.ix"))
        ys.append(md.NewIntervalVar(y - bo, h + bo + t, y_end, f"{iid}.iy"))
        pads: dict[str, tuple] = {}
        for k, name in enumerate(it.get("pad_names", [])):
            dx = sum(bi * units(2 * r["pads_mm"][k][0]) for bi, r in zip(b, rots))
            dy = sum(bi * units(2 * r["pads_mm"][k][1]) for bi, r in zip(b, rots))
            pads[name] = (cx2 + dx, cy2 + dy)
        items[iid] = {"b": b, "x": x, "y": y, "w": w, "h": h, "c2": (cx2, cy2), "pads": pads, "rots": rots}
        for bi in b[1:]:
            costs.append((m["weights"]["rotation"], bi))

    for o in m.get("obstacles", []):
        (ox0, oy0), (ox1, oy1) = (o["lo_mm"][0] - ox, o["lo_mm"][1] - oy), (o["hi_mm"][0] - ox, o["hi_mm"][1] - oy)
        lo_x, hi_x = units(ox0 - hg), units(ox1 + hg)
        lo_y, hi_y = units(oy0 - hg), units(oy1 + hg)
        # Present unless we are asking which declarations conflict.
        on = guard(o.get("from", "an obstacle on the board"))
        xs.append(md.NewOptionalIntervalVar(lo_x, hi_x - lo_x, hi_x, on, "obstacle.x"))
        ys.append(md.NewOptionalIntervalVar(lo_y, hi_y - lo_y, hi_y, on, "obstacle.y"))
    md.AddNoOverlap2D(xs, ys)

    for a in m.get("apart", []):
        p, q, d = items[a["a"]], items[a["b"]], units(a["mm"])
        side = [md.NewBoolVar("") for _ in range(4)]
        md.Add(p["x"] >= q["x"] + q["w"] + d).OnlyEnforceIf(side[0])
        md.Add(q["x"] >= p["x"] + p["w"] + d).OnlyEnforceIf(side[1])
        md.Add(p["y"] >= q["y"] + q["h"] + d).OnlyEnforceIf(side[2])
        md.Add(q["y"] >= p["y"] + p["h"] + d).OnlyEnforceIf(side[3])
        md.AddBoolOr(side).OnlyEnforceIf(guard(a["from"]))

    def point(ref: dict):
        """A point an item or pad sits at: twice its coordinates, in grid units."""
        if "at_mm" in ref:
            return units(2 * (ref["at_mm"][0] - ox)), units(2 * (ref["at_mm"][1] - oy))
        it = items[ref["item"]]
        return it["pads"][ref["pad"]] if "pad" in ref else it["c2"]

    def dist(a, b, tag: str):
        """L1 distance between two points, as a variable the objective can weigh."""
        out = []
        for k in range(2):
            v = md.NewIntVar(0, big, f"{tag}.{k}")
            md.AddAbsEquality(v, a[k] - b[k])
            out.append(v)
        return out[0] + out[1]

    for n in m.get("near", []):
        frm = {"item": n["item"], **({"pad": n["pad"]} if "pad" in n else {})}
        costs.append((n["weight"], dist(point(frm), point(n["to"]), f"near.{n['item']}")))

    for net in m.get("nets", []):
        ends = [point(e) for e in net["pads"]]
        if len(ends) < 2:
            continue
        for k in range(2):
            lo, hi = md.NewIntVar(-big, big, ""), md.NewIntVar(-big, big, "")
            md.AddMinEquality(lo, [e[k] for e in ends])
            md.AddMaxEquality(hi, [e[k] for e in ends])
            costs.append((net["weight"], hi - lo))

    md.Minimize(sum(wt * e for wt, e in costs))
    return md, items, guards


def solver(effort: float) -> cp_model.CpSolver:
    """One worker, a fixed seed, a deterministic budget: same model, same answer."""
    s = cp_model.CpSolver()
    s.parameters.num_workers = 1
    s.parameters.random_seed = 0
    s.parameters.max_deterministic_time = effort
    return s


def snapshot(s: cp_model.CpSolver, items: dict) -> dict:
    """Each item's corner, turn and doubled centre in a solution."""
    out = {}
    for iid, it in items.items():
        k = next(i for i, bi in enumerate(it["b"]) if s.Value(bi))
        out[iid] = (s.Value(it["x"]), s.Value(it["y"]), k, s.Value(it["c2"][0]), s.Value(it["c2"][1]))
    return out


def solve(m: dict, hint: dict | None = None):
    """A first placement, then better ones a neighbourhood at a time.

    One solver worker finds a placement; then, for each item in turn, it and
    its nearest few are freed while everything else stays put, and the
    solver improves that neighbourhood (large neighbourhood search). Every
    step is single-threaded with a deterministic budget, so the whole search
    is deterministic. It stops when a full pass improves nothing or the
    budget (`effort`) is spent.
    """
    budget = float(m.get("effort", 20.0))
    hood = int(m.get("neighbourhood", 4))
    md, items, _ = build(m, explain=False)
    # Start from the last answer where it still fits: a small change to the
    # declaration then moves little, and the search starts near a good one.
    g = m["grid_mm"]
    ox, oy = m["board"]["lo_mm"]
    for iid, h in (hint or {}).items():
        it = items.get(iid)
        if it is None:
            continue
        k = next((i for i, r in enumerate(it["rots"]) if abs(r["deg"] - h["rotation_deg"]) < 1e-6), None)
        if k is None:
            continue
        w, d = it["rots"][k]["size_mm"]
        md.AddHint(it["x"], round((h["centre_mm"][0] - w / 2 - ox) / g))
        md.AddHint(it["y"], round((h["centre_mm"][1] - d / 2 - oy) / g))
        for i, bi in enumerate(it["b"]):
            md.AddHint(bi, int(i == k))
    # Any placement first — improving one is what the budget is for.
    s = solver(budget / 2)
    s.parameters.stop_after_first_solution = True
    s.parameters.repair_hint = bool(hint)
    status = s.Solve(md)
    spent = s.ResponseProto().deterministic_time
    if status not in (cp_model.OPTIMAL, cp_model.FEASIBLE):
        return status, None, None
    best, cost = snapshot(s, items), s.ObjectiveValue()
    ids = sorted(items)
    # Who shares a net with whom: a circuit moves together.
    linked: dict[str, set] = {i: set() for i in ids}
    for net in m.get("nets", []):
        on = {e["item"] for e in net["pads"]}
        for i in on:
            linked[i] |= on - {i}
    improved, rounds = True, 0
    while (improved or rounds < 2) and spent < budget:
        improved = False
        for anchor in ids:
            if spent >= budget:
                break
            ax, ay = best[anchor][3], best[anchor][4]
            near = lambda j: (abs(best[j][3] - ax) + abs(best[j][4] - ay), j)  # noqa: E731
            # Alternate passes: what is beside it, and what it connects to.
            pool = sorted(linked[anchor], key=near) if rounds % 2 else []
            size = hood + len(pool)
            pool = [anchor] + pool + sorted(ids, key=near)
            free = set(list(dict.fromkeys(pool))[:size])
            md, its, _ = build(m, explain=False)
            for j, it in its.items():
                x, y, k, _, _ = best[j]
                if j in free:
                    md.AddHint(it["x"], x)
                    md.AddHint(it["y"], y)
                    for i, bi in enumerate(it["b"]):
                        md.AddHint(bi, int(i == k))
                else:
                    md.Add(it["x"] == x)
                    md.Add(it["y"] == y)
                    md.Add(it["b"][k] == 1)
            s = solver(min(0.25 * size / hood, budget - spent))
            st = s.Solve(md)
            spent += s.ResponseProto().deterministic_time
            if st in (cp_model.OPTIMAL, cp_model.FEASIBLE) and s.ObjectiveValue() < cost - 0.5:
                best, cost = snapshot(s, its), s.ObjectiveValue()
                improved = True
        rounds += 1
    return cp_model.FEASIBLE, best, cost


def conflict(m: dict) -> list[str]:
    """A minimal set of declarations that cannot hold together.

    The solver's own core is sufficient but rarely minimal; each declaration
    in it is dropped in turn, and stays dropped when what is left still
    cannot hold. What remains needs every member: relax any one and the
    rest can hold (as far as the effort allows a proof).
    """
    effort = float(m.get("effort", 20.0))
    md, _, guards = build(m, explain=True)
    md.AddAssumptions([lit for lit, _ in guards])
    s = solver(effort)
    if s.Solve(md) != cp_model.INFEASIBLE:
        return []
    core = set(s.SufficientAssumptionsForInfeasibility())
    keep = [why for lit, why in guards if lit.Index() in core]
    for why in sorted(set(keep)):
        trial = [w for w in keep if w != why]
        md, _, guards = build(m, explain=True)
        md.AddAssumptions([lit for lit, w in guards if w in trial])
        if solver(effort / 4).Solve(md) == cp_model.INFEASIBLE:
            keep = trial
    return sorted(set(keep))


def main() -> int:
    m = json.load(sys.stdin)
    # fid sends {"model": …, "hint": <the previous answer's items>}; by hand
    # it is the model alone.
    hint = None
    if "model" in m:
        m, hint = m["model"], m.get("hint")
    status, best, cost = solve(m, hint)
    if status == cp_model.INFEASIBLE:
        json.dump({"status": "infeasible", "conflict": conflict(m)}, sys.stdout, indent=1)
        print()
        return 0
    if best is None:
        json.dump({"status": "unknown", "conflict": []}, sys.stdout, indent=1)
        print()
        return 0
    g = m["grid_mm"]
    ox, oy = m["board"]["lo_mm"]
    rots = {it["id"]: it["rotations"] for it in m["items"]}
    out = {
        iid: {
            "centre_mm": [round(ox + cx2 * g / 2, 4), round(oy + cy2 * g / 2, 4)],
            "rotation_deg": rots[iid][k]["deg"],
        }
        for iid, (_, _, k, cx2, cy2) in best.items()
    }
    json.dump(
        {
            "status": "optimal" if status == cp_model.OPTIMAL else "feasible",
            "objective": round(cost),
            "items": out,
        },
        sys.stdout,
        indent=1,
        sort_keys=True,
    )
    print()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
