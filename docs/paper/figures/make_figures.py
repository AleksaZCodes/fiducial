"""Draw the paper's figures.

    python3 docs/paper/figures/make_figures.py

Plain on purpose: black lines on white, Times New Roman as the template
prescribes, one accent colour for what goes wrong, and no label a caption
already says. Every figure is placed one column wide (3.3 in); the SVG's
1000 units are that width, so 34 units is 8 pt in print, the template's
minimum for figure labels. No label here is smaller.

Fig. 1 and Fig. 2 are drawn here. Fig. 3 is cropped from the sensor stick's
renders (examples/sensor-stick/hardware/showcase/). Fig. 4 is the text of two
real gate failures, taken from docs/paper/markers/results.json.
"""

import json
from pathlib import Path

import cairosvg
from PIL import Image

HERE = Path(__file__).parent
REPO = HERE.parents[2]
SCALE = 2.4  # 1000 units → 2400 px, about 700 dpi at column width

SERIF = "Times New Roman"
MONO = "Liberation Mono"
INK = "#1a1a1a"
GREY = "#6b6b6b"
RED = "#b3261e"
LINE = 2.2


def svg(w, h, body):
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
<defs>
  <marker id="a" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
    <path d="M0,0 L10,5 L0,10 z" fill="{INK}"/>
  </marker>
  <marker id="r" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
    <path d="M0,0 L10,5 L0,10 z" fill="{RED}"/>
  </marker>
</defs>
<rect width="{w}" height="{h}" fill="white"/>
{body}
</svg>"""


def box(x, y, w, h, fill="white", stroke=INK, dash=None):
    d = f' stroke-dasharray="{dash}"' if dash else ""
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{fill}" stroke="{stroke}" stroke-width="{LINE}"{d}/>'


def text(x, y, s, size=34, family=SERIF, anchor="middle", fill=INK, weight="normal", style="normal"):
    s = s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
    return (f'<text x="{x}" y="{y}" font-family="{family}" font-size="{size}" fill="{fill}" '
            f'text-anchor="{anchor}" font-weight="{weight}" font-style="{style}">{s}</text>')


def arrow(x1, y1, x2, y2, colour=INK, marker="a"):
    return (f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{colour}" '
            f'stroke-width="{LINE}" marker-end="url(#{marker})"/>')


def path_arrow(d, colour=INK, marker="a"):
    return f'<path d="{d}" fill="none" stroke="{colour}" stroke-width="{LINE}" marker-end="url(#{marker})"/>'


def render(name, markup):
    (HERE / f"{name}.svg").write_text(markup)
    cairosvg.svg2png(bytestring=markup.encode(), write_to=str(HERE / f"{name}.png"), scale=SCALE)


# ── Fig. 1: one fact, copied or declared ─────────────────────────────────────


def fig1():
    """(a) and (b) side by side, so the figure is short enough for the foot
    of the first column."""
    outputs = ["schematic", "board", "firmware", "web page"]
    step, bh = 60, 48            # an output box every 60 units, 48 tall
    top = 52                     # below the panel letters
    mid = top + (3 * step + bh) / 2

    def panel(x0, letter, src_fill, src_label, bad):
        b = [text(x0, 34, letter, anchor="start")]
        b.append(box(x0, mid - 46, 170, 92, fill=src_fill))
        b.append(text(x0 + 85, mid - 8, src_label))
        b.append(text(x0 + 85, mid + 32, "GPIO 4", family=MONO))
        for i, name in enumerate(outputs):
            y = top + i * step
            wrong = bad and i == 2
            c = RED if wrong else INK
            b.append(box(x0 + 232, y, 240, bh, stroke=c))
            b.append(text(x0 + 244, y + 34, name, anchor="start", fill=c))
            b.append(text(x0 + 460, y + 34, "8" if wrong else "4", family=MONO,
                          anchor="end", fill=c))
            b.append(arrow(x0 + 170, mid, x0 + 228, y + bh / 2))
        return b

    a = panel(6, "(a)", "white", "SDA pin", True)
    bb = panel(520, "(b)", "#f2f2f2", "declared", False)
    end = top + 3 * step + bh
    a.append(text(242, end + 42, "copied by hand", style="italic"))
    bb.append(text(756, end + 42, "derived, checked on commit", style="italic"))
    render("fig1-one-fact", svg(1000, end + 56, "\n".join(a + bb)))


# ── Fig. 2: the loop ─────────────────────────────────────────────────────────


def fig2():
    b = []
    W, H = 340, 120
    L, R = 40, 620  # left and right column x
    T, B = 40, 400  # top and bottom row y
    b.append(box(L, T, W, H))
    b.append(text(L + W / 2, T + 52, "person"))
    b.append(text(L + W / 2, T + 94, "and AI agents"))
    b.append(box(R, T, W, H, fill="#f2f2f2"))
    b.append(text(R + W / 2, T + 72, "declarations"))
    b.append(box(R, B, W, H))
    b.append(text(R + W / 2, B + 72, "artifacts"))
    b.append(box(L, B, W, H))
    b.append(text(L + W / 2, B + 72, "checks"))
    # edit →
    b.append(arrow(L + W, T + H / 2, R - 4, T + H / 2))
    b.append(text((L + W + R) / 2, T + H / 2 - 16, "edit", style="italic", size=34))
    # derive ↓ (engines beside it)
    b.append(arrow(R + W / 2, T + H, R + W / 2, B - 4))
    b.append(text(R + W / 2 - 20, (T + H + B) / 2 + 12, "derive", style="italic", size=34, anchor="end"))
    b.append(box(R + W / 2 + 26, (T + H + B) / 2 - 62, 150, 124, dash="8 6", stroke=GREY))
    for i, s in enumerate(["solver", "CAD", "EDA"]):
        b.append(text(R + W / 2 + 101, (T + H + B) / 2 - 22 + i * 38, s, size=34, fill=GREY))
    # ← checked
    b.append(arrow(R, B + H / 2, L + W + 4, B + H / 2))
    b.append(text((L + W + R) / 2, B + H / 2 - 16, "check", style="italic", size=34))
    # fail ↑ back to the person, with the message
    b.append(arrow(L + W / 2, B, L + W / 2, T + H + 4, colour=RED, marker="r"))
    b.append(text(L + W / 2 + 18, (T + H + B) / 2 - 6, "fails, naming", size=34, anchor="start", fill=RED))
    b.append(text(L + W / 2 + 18, (T + H + B) / 2 + 32, "the declaration", size=34, anchor="start", fill=RED))
    # pass ↓
    b.append(arrow(L + W / 2, B + H, L + W / 2, B + H + 86))
    b.append(text(L + W / 2 + 18, B + H + 60, "passes: merge", style="italic", size=34, anchor="start"))
    render("fig2-loop", svg(1000, B + H + 100, "\n".join(b)))


# ── Fig. 3: the sensor stick, from its renders ───────────────────────────────


def whiten(im, bg=(242, 239, 233), tol=6):
    im = im.convert("RGB")
    px = im.load()
    for y in range(im.height):
        for x in range(im.width):
            p = px[x, y]
            if all(abs(p[i] - bg[i]) <= tol for i in range(3)):
                px[x, y] = (255, 255, 255)
    return im


def fig3():
    show = REPO / "examples/sensor-stick/hardware/showcase"
    closed = whiten(Image.open(show / "assembled.png")).crop((400, 270, 1160, 810))
    board = whiten(Image.open(show / "board.png")).crop((330, 60, 1230, 1000))
    h = 900
    closed = closed.resize((round(closed.width * h / closed.height), h), Image.LANCZOS)
    board = board.resize((round(board.width * h / board.height), h), Image.LANCZOS)
    gap = 60
    label_h = 96
    out = Image.new("RGB", (closed.width + gap + board.width, h + label_h), "white")
    out.paste(closed, (0, label_h))
    out.paste(board, (closed.width + gap, label_h))
    # panel letters, drawn in the same serif as the other figures
    letters = svg(out.width, label_h,
                  text(10, 70, "(a)", size=80, anchor="start")
                  + text(closed.width + gap + 10, 70, "(b)", size=80, anchor="start"))
    tmp = HERE / "_letters.png"
    cairosvg.svg2png(bytestring=letters.encode(), write_to=str(tmp))
    out.paste(Image.open(tmp).convert("RGB"), (0, 0))
    tmp.unlink()
    out.save(HERE / "fig3-sensor-stick.png")


# ── Fig. 4: what a failure says ──────────────────────────────────────────────


def fig4():
    import textwrap

    res = json.loads((REPO / "docs/paper/markers/results.json").read_text())
    by_id = {r["id"]: r for r in res["results"]}
    b, y = [], 10
    # (marker, message lines kept, cut the last kept line before this text)
    for letter, mid, keep, cut in (("(a)", "W1", 4, None),
                                   ("(b)", "H7", 2, " or mark the line")):
        gate = next(g for g in by_id[mid]["gates"] if not g["ok"])
        out = gate["output"].splitlines()
        # The failing pipeline's own lines: from its ✗ to the next pipeline.
        i = next(k for k, l in enumerate(out) if "✗" in l)
        msg = []
        for l in out[i + 1:]:
            if l.strip().startswith("▶"):
                break
            msg.append(l.strip().removeprefix("error: "))
        msg = msg[:keep]
        if cut and cut in msg[-1]:
            msg[-1] = msg[-1][: msg[-1].index(cut)] + " …"
        lines = []
        for l in msg:
            lines += textwrap.wrap(l, 46, subsequent_indent="  ") or [""]
        b.append(text(10, y + 34, letter, anchor="start"))
        top = y + 50
        hgt = 40 + 42 * (len(lines) + 1)
        b.append(box(10, top, 980, hgt, fill="#f7f7f7", stroke=GREY))
        b.append(text(30, top + 50, "$ fid derive", family=MONO, anchor="start", fill=GREY))
        for k, l in enumerate(lines):
            b.append(text(30, top + 92 + 42 * k, l, family=MONO, anchor="start",
                          fill=RED if k == 0 else INK))
        y = top + hgt + 16
    render("fig4-failures", svg(1000, y, "\n".join(b)))


if __name__ == "__main__":
    fig1()
    fig2()
    fig3()
    if (REPO / "docs/paper/markers/results.json").exists():
        fig4()
    print("figures written to", HERE)
