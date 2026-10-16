"""Build the Fiducial paper (.docx) on the IEEESTEC template.

The template is the author's own IEEESTEC 2025 paper (Outreach), converted
from .doc to .docx: its styles carry the conference formatting (two columns,
auto-numbered headings, "Fig. N." captions, "TABLE N." heads, "[N]"
references). Every paragraph after the author block is removed and the text
below is written in its place.

    python3 build_paper.py TEMPLATE.docx FIGURE_DIR OUT.docx
"""

import copy
import sys

import docx
from docx.enum.table import WD_TABLE_ALIGNMENT
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Inches

TEMPLATE, FIGS, OUT = sys.argv[1:4]

# Only the conference line changes; the DOI and "Original scientific paper"
# lines stay as the template has them.
HEADER = ["IEEESTEC – 19th Student Projects Conference, Niš, 2026."]

TITLE = "Fiducial: Build Any Robust Multidisciplinary Product with AI Agents"

AUTHOR = [
    "Aleksa Zdravković",
    "12th grade high school student",
    "ETŠ “Nikola Tesla” Niš",
    "Niš, Serbia",
    "aleksazdravkovic@proton.me",
    "ORCID: 0009-0004-1535-1839",
]

import json as _json
import re as _re
from pathlib import Path as _Path

_HERE = _Path(__file__).resolve().parent
_RESULTS = _json.loads((_HERE / "markers/results.json").read_text())
_SUM = _RESULTS["summary"]
_BY_ID = {r["id"]: r for r in _RESULTS["results"]}
VERSION = _re.search(r'^version\s*=\s*"([^"]+)"',
                     (_HERE.parents[1] / "Cargo.toml").read_text(), _re.M).group(1)

def _count(gate):
    """How many markers a gate caught first."""
    return sum(1 for r in _RESULTS["results"]
               if not r["control"] and r["caught_by"] and r["caught_by"][0] == gate)


_C1 = _BY_ID["C1"]

ALL_CAUGHT = _SUM["markers_caught"] == _SUM["markers"]
CONTROLS_OK = _SUM["controls_passed"] == _SUM["controls"]

_PIN_S = round(next(g["seconds"] for g in _C1["gates"] if g["gate"] == "derive"))
_PIN_N = len([f for f in _C1["changed"] if "/generated/" in f])
_N = _SUM["markers_caught"]
_ALL = "all 20" if ALL_CAUGHT else f"{_N} of the 20"

ABSTRACT = (
    "This paper presents Fiducial, an open-source platform for building "
    "products that combine software and hardware with AI coding agents. "
    "Every fact about a product is declared once in typed text files, every "
    "artifact that uses it – code, translations, circuit board, case – is "
    "derived from it by one command, and checks on every commit refuse "
    "anything that no longer matches its declaration. In a test of 20 "
    "realistic mistakes in a web product and a USB-C sensor stick, the first "
    "version caught 15. Each miss was turned into a general rule, and the "
    f"current version catches {_ALL}."
)

KEYWORDS = (
    "AI agents, single source of truth, code generation, consistency "
    "checking, hardware–software co-design, continuous integration"
)

# ("h1", text) | ("h2", text) | ("p", text) | ("fig", file[@inches], caption)
# | ("table", caption, header, rows, widths) — a row that is a str is a group
BODY = [
    ("h1", "Introduction"),
    ("p",
     "While building Outreach [1], an off-grid messaging system made of "
     "radios, gateways, a companion app and printed cases, the author met "
     "the same problem more than once. The firmware and the app disagreed on "
     "the layout of a message, and a printed case did not fit the board it "
     "was made for. Neither part was wrong on its own. The facts they shared "
     "had been copied by hand between different tools, one copy changed, and "
     "the mistake showed up only when the parts were put together."),
    ("p",
     "AI coding agents [2], [3] now write much of this work, but they make "
     "the problem faster rather than smaller: an agent that knows one "
     "codebase does not know what changed in the others. Every project also "
     "has its own layout and habits, so each new product starts from scratch."),
    ("p",
     "Fiducial [4] gives every product the same structure, built on one rule: "
     "each fact is declared once, everything that uses it is derived from it, "
     "and every change is checked (Fig. 1). People and agents edit the "
     "declarations, never the outputs. The idea builds on model-driven "
     "engineering [5] and on build systems that track their inputs by hash "
     "[6], applied to every discipline of one product at once, in plain files "
     "that agents read and write well."),
    ("fig", "fig1-one-fact.png",
     "One fact in four places: (a) copied by hand, one copy changes and "
     "nothing notices; (b) declared once, every copy is derived and checked."),

    ("h1", "System Overview and Implementation"),
    ("p",
     "Fiducial is a command-line tool, fid, written in Rust. fid new creates "
     "a product repository with a fixed layout, and fid add enables "
     "capabilities – packaged pipelines for web applications, translations, "
     "legal pages, design systems, databases, firmware, hardware and more. "
     "Because every product shares one structure, a new one starts from "
     "working, checked parts. Fig. 2 shows how a change moves through it."),
    ("fig", "fig2-loop.png@2.9",
     "A change in Fiducial: declarations are edited, derivation produces the "
     "artifacts with help from established engines, and the checks either "
     "accept the change or name the declaration at fault."),
    ("h2", "Declarations"),
    ("p",
     "A declaration is a typed text file in version control: the strings of "
     "each language, the brand and pages, the colour palette, the messages a "
     "device sends, and the parts of a physical product with what each pin "
     "connects to. Text files were chosen over a modelling tool because "
     "coding agents read and write them well, and every change goes through "
     "a normal code review. A declaration holds facts only; nothing in it "
     "says where a part sits on the board or how a page is laid out."),
    ("h2", "Derivation"),
    ("p",
     "fid derive runs every pipeline. Each one reads the declarations it "
     "needs and writes its artifacts, such as a typed translation module, a "
     "sitemap, a firmware pin map, TypeScript types or a KiCad board [7]. The "
     "hash of every input and output is stored in a lock file. Derivation is "
     "deterministic: the same inputs always give the same bytes, so a changed "
     "hash always means a real change."),
    ("h2", "Engines for Hard Problems"),
    ("p",
     "Some derivations are real engineering problems. The first version "
     "placed board parts with a hand-written search, and each product added "
     "special rules until they began to interact. Now the platform only "
     "writes the problem down and hands it to an established engine: the "
     "CP-SAT solver [8] places the parts, build123d on the OpenCascade kernel "
     "[9] builds the case, and KiCad routes and checks the board. Every "
     "placement rule is one of eight generic kinds, so a new requirement is a "
     "new declaration, not new code."),
    ("h2", "Checks"),
    ("p",
     "Three kinds of check run on every commit. fid derive --check fails when "
     "a declaration changed without its outputs being re-derived, or when an "
     "output was edited by hand. Derivation refuses declarations that break a "
     "rule: a missing translation, too little colour contrast, parts over the "
     "cost limit, a board that does not fit its case. Compilers and type "
     "checkers read the derived files, so code that still uses a renamed fact "
     "stops compiling. Each failure names the file and the declaration at "
     "fault, which is the signal an agent needs to correct itself."),
    ("h2", "Working with AI Agents"),
    ("p",
     "Every product carries an AGENTS.md file [10] and one instruction file "
     "per capability, so any agent starts from the same context. Agents are "
     "told to fix a problem where the fact is declared, never in a generated "
     "file. When a mistake gets past the checks, it is not patched for that "
     "one product; it becomes a general rule for all of them."),
    ("h2", "Worked Example: Sensor Stick"),
    ("p",
     "The repository includes a public example: a USB-C stick that reports "
     "temperature and humidity to a web page (Fig. 3). It has a printed case, "
     "a two-layer board with 23 parts, Rust firmware and a web page. The "
     "person writes the product declaration, an outline drawing, the firmware "
     "and the page. Fiducial derives the board, the case, the manufacturing "
     "files, and the code both sides use to read the sensor’s messages."),
    ("fig", "fig3-sensor-stick.png",
     "The sensor stick, derived from its declaration: (a) the closed case; "
     "(b) the routed board inside it, with the sensor under its vent."),

    ("h1", "Results"),
    ("p",
     "Fiducial was built in six weeks by one person working with AI agents "
     "and used on two products, the sensor stick and a private one. The "
     "sensor stick’s board passes KiCad’s design-rule check with no errors. "
     "Moving its sensor to another pin is one changed line: fid derive then "
     f"rewrites {_PIN_N} generated files in {_PIN_S} s, including the board, "
     "the firmware’s pin map and the page’s types, and neither the firmware "
     "nor the page needs an edit."),
    ("p",
     "To test whether the checks catch the mistakes people actually make, 20 "
     "realistic mistakes were injected one at a time, in the manner of "
     "mutation testing [11]: ten in a fresh web product in English and "
     "Serbian, and ten in the sensor stick’s hardware, firmware and "
     "interface. They cover eight kinds of drift (Table I). After each one "
     "the checks ran as in continuous integration, and the product was reset. "
     "Three correct changes and two runs with no change confirmed that the "
     "checks do not fail without a reason. The test is a script in the "
     "repository."),
    ("table", "Injected Mistakes, by Kind of Drift",
     ("Kind of drift", "Example", "0.9.4", "Now"), "TABLE1",
     (1400, 1900, 600, 600)),
    ("p", "FINDINGS"),
    ("fig", "fig4-failures.png",
     "Two failures as the person or agent sees them: (a) a string added in "
     "English only; (b) firmware naming a pin directly."),
    ("p",
     "The mistakes were chosen by the author of the platform, only two "
     "products were built, and neither has been manufactured yet. The test "
     "measures the checks, not whether agents make fewer mistakes with "
     "Fiducial than without it."),

    ("h1", "Conclusion"),
    ("p",
     "Better models will move the line between what a person checks and what "
     "is left to agents, but a probabilistic system cannot guarantee its own "
     "output. Fiducial keeps that output bounded: each fact is declared once, "
     "everything else is derived, and every change is checked. With it, one "
     "person working with AI agents kept a product’s code, board and case in "
     "agreement, and mistakes were caught early by messages that name their "
     "cause. Future work will "
     "manufacture and test the sensor stick and compare agents working with "
     "and without the platform on the same tasks. Fiducial and the example "
     "are open source [4]."),
]

# Table I: the twenty by kind, with an example of each and how many were
# caught on 0.9.4 (the first run) and on this version.
_KINDS = [
    ("Stale output", "Domain edited, not re-derived", ["W2", "W6", "H1"], 1),
    ("Generated file edited", "Pin map edited by hand", ["W3", "W7", "W9", "H2", "H8"], 5),
    ("Missing declaration", "String in English only", ["W1", "W4", "W5"], 3),
    ("Rule broken", "Contrast below 4.5 : 1", ["W8", "H3", "H5"], 3),
    ("Declarations conflict", "Case too narrow for board", ["H4", "H10"], 2),
    ("Code out of date", "Net renamed, old name used", ["H6"], 1),
    ("Declaration bypassed", "Pin typed as a number", ["H7", "H9"], 0),
    ("Not translated", "Serbian left in English", ["W10"], 0),
]
TABLE1 = []
for kind, example, ids, first in _KINDS:
    now = sum(1 for i in ids if _BY_ID[i]["pass"])
    TABLE1.append((kind, example, f"{first}/{len(ids)}", f"{now}/{len(ids)}"))
TABLE1.append(("Total", "", f"{sum(k[3] for k in _KINDS)}/20", f"{_N}/20"))

FINDINGS = (
    "On release 0.9.4 the checks caught 15 of the 20. Each miss was a fact "
    "the platform did not yet own: two declarations edited without "
    "re-deriving went unnoticed, because only the hardware pipeline tracked "
    "its inputs; firmware could name a pin or a sensor address directly; and "
    "a Serbian string left in English passed every structural check. Each "
    f"miss became a general rule. On release {VERSION}, "
    + ("all 20 are caught" if ALL_CAUGHT else f"{_N} of the 20 are caught")
    + f" – {_count('derive')} during derivation, {_count('--check')} by "
    f"--check and {_N - _count('derive') - _count('--check')} by the "
    "compiler – "
    + ("and every correct change passes. " if CONTROLS_OK else
       f"and {_SUM['controls_passed']} of the {_SUM['controls']} controls pass. ")
    + "Fig. 4 shows two of the messages."
)

REFERENCES = [
    "A. Zdravković, Đ. Jovanović and P. Šubarević, “Outreach: Modular "
    "AI-enhanced off-grid mesh communication system for remote "
    "environments,” IEEESTEC – 18th Student Projects Conference, Niš, 2025.",
    "Claude Code documentation. Available at: "
    "https://docs.anthropic.com/en/docs/claude-code",
    "OpenAI Codex. Available at: https://openai.com/codex/",
    f"A. Zdravković, Fiducial, release {VERSION}, 2026. Available at: "
    "https://github.com/AleksaZCodes/fiducial",
    "D. C. Schmidt, “Guest editor’s introduction: Model-driven "
    "engineering,” Computer, vol. 39, no. 2, pp. 25–31, 2006.",
    "E. Dolstra, M. de Jonge and E. Visser, “Nix: A safe and policy-free "
    "system for software deployment,” in Proc. LISA, 2004, pp. 79–92.",
    "KiCad EDA. Available at: https://www.kicad.org/",
    "L. Perron and F. Didier, CP-SAT, Google OR-Tools. Available at: "
    "https://developers.google.com/optimization/cp/cp_solver",
    "build123d, a Python CAD library on the OpenCascade kernel. Available "
    "at: https://github.com/gumyr/build123d",
    "AGENTS.md, a format for guiding coding agents. Available at: "
    "https://agents.md/",
    "R. A. DeMillo, R. J. Lipton and F. G. Sayward, “Hints on test data "
    "selection: Help for the practicing programmer,” Computer, vol. 11, "
    "no. 4, pp. 34–41, 1978.",
]

FIG_WIDTH = Inches(3.15)  # the text column is 3.19 in; anything wider is cut at its edge

d = docx.Document(TEMPLATE)
styles = {s.name.lower(): s for s in d.styles}


def style(name):
    return styles[name.lower()]


def set_text(p, text):
    """Replace a paragraph's text, keeping the first run's formatting."""
    runs = p.runs
    if not runs:
        p.add_run(text)
        return
    runs[0].text = text
    for r in runs[1:]:
        r._r.getparent().remove(r._r)


# Header: the conference line (first-page header).
for s in d.sections:
    for hf in (s.first_page_header, s.header):
        if hf.is_linked_to_previous:
            continue
        for p in hf.paragraphs:
            if p.text.startswith("IEEESTEC"):
                set_text(p, HEADER[0])

body = d.element.body
children = list(body.iterchildren())

# Title.
title = next(p for p in d.paragraphs if p.style is not None
             and p.style.name == "paper title")
set_text(title, TITLE)

# Author block: one author, one cell, no borders.
tbl = d.tables[0]
tr = tbl.rows[0]._tr
tcs = tr.findall(qn("w:tc"))
for tc in tcs[1:]:
    tr.remove(tc)
grid = tbl._tbl.find(qn("w:tblGrid"))
cols = grid.findall(qn("w:gridCol"))
for c in cols[1:]:
    grid.remove(c)
total = 10336
cols[0].set(qn("w:w"), str(total))
tc = tcs[0]
tcw = tc.find(qn("w:tcPr")).find(qn("w:tcW"))
tcw.set(qn("w:w"), str(total))
borders = tc.find(qn("w:tcPr")).find(qn("w:tcBorders"))
if borders is not None:
    for b in list(borders):
        b.set(qn("w:val"), "nil")
cell_paras = tbl.rows[0].cells[0].paragraphs
for p, text in zip(cell_paras, AUTHOR):
    set_text(p, text)

# Everything from the abstract to the last reference goes.
abstract_el = next(el for el in children if el.tag == qn("w:p")
                   and "Abstract" in "".join(el.itertext()))
start = children.index(abstract_el)
last_sect = max(i for i, el in enumerate(children)
                if el.tag == qn("w:p") and el.find(".//" + qn("w:sectPr")) is not None)
anchor = children[last_sect]
for el in children[start:last_sect]:
    body.remove(el)


def para(style_name, text=None, align=None):
    p = d.add_paragraph(style=style(style_name))
    if text is not None:
        p.add_run(text)
    if align is not None:
        p.alignment = align
    anchor.addprevious(p._p)
    return p


def labelled(style_name, label, text):
    """Abstract—… / Keywords—…: bold-italic label, as in the template."""
    p = para(style_name)
    r = p.add_run(label + "—")
    r.italic = True
    p.add_run(text)
    return p


labelled("Abstract", "Abstract", ABSTRACT)
labelled("Keywords", "Keywords", KEYWORDS)


def keep_with_next(p):
    p.paragraph_format.keep_with_next = True


def add_table(caption, header, rows, widths):
    """A table; a row that is a plain string is a group heading across it.

    Widths are twips; the two-column body is 4860 twips (3.375 in) wide, and
    cell margins take what the widths leave."""
    keep_with_next(para("table head", caption))
    ncols = len(header)
    t = d.add_table(rows=1 + len(rows), cols=ncols)
    t.alignment = WD_TABLE_ALIGNMENT.CENTER
    tblPr = t._tbl.tblPr
    tblW = tblPr.find(qn("w:tblW"))
    tblW.set(qn("w:w"), str(sum(widths)))
    tblW.set(qn("w:type"), "dxa")
    layout = OxmlElement("w:tblLayout")
    layout.set(qn("w:type"), "fixed")
    tblPr.append(layout)
    for gc, w in zip(t._tbl.tblGrid.findall(qn("w:gridCol")), widths):
        gc.set(qn("w:w"), str(w))
    for row_i, values in enumerate([header] + list(rows)):
        cells = t.rows[row_i].cells
        if isinstance(values, str):
            cell = cells[0].merge(cells[-1])
            p = cell.paragraphs[0]
            p.style = style("table copy")
            p.alignment = WD_ALIGN_PARAGRAPH.LEFT
            r = p.add_run(values)
            r.italic = True
            continue
        for col_i, value in enumerate(values):
            cell = cells[col_i]
            tcPr = cell._tc.get_or_add_tcPr()
            tcW = tcPr.find(qn("w:tcW"))
            if tcW is None:
                tcW = OxmlElement("w:tcW")
                tcPr.append(tcW)
            tcW.set(qn("w:w"), str(widths[col_i]))
            tcW.set(qn("w:type"), "dxa")
            p = cell.paragraphs[0]
            p.style = style("table col head" if row_i == 0 else "table copy")
            p.alignment = WD_ALIGN_PARAGRAPH.LEFT
            # "--check" must not break at its hyphens.
            p.add_run(value.replace("--", "\u2011\u2011"))
    for row in t.rows[:-1]:
        for cell in row.cells:
            for cp in cell.paragraphs:
                cp.paragraph_format.keep_with_next = True
    for row in t.rows:
        trPr = row._tr.get_or_add_trPr()
        trPr.append(OxmlElement("w:cantSplit"))
    b = OxmlElement("w:tblBorders")
    for edge in ("top", "left", "bottom", "right", "insideH", "insideV"):
        e = OxmlElement("w:" + edge)
        e.set(qn("w:val"), "single")
        e.set(qn("w:sz"), "4")
        e.set(qn("w:space"), "0")
        e.set(qn("w:color"), "000000")
        b.append(e)
    tblPr.append(b)
    anchor.addprevious(t._tbl)
    # a small gap after the table
    para("Body Text", "")


for item in BODY:
    kind = item[0]
    if kind == "h1":
        keep_with_next(para("Heading 1", item[1]))
    elif kind == "h2":
        keep_with_next(para("Heading 2", item[1]))
    elif kind == "p":
        para("Body Text", FINDINGS if item[1] == "FINDINGS" else item[1])
    elif kind == "fig":
        _, name, caption = item
        p = para("Body Text", align=WD_ALIGN_PARAGRAPH.CENTER)
        p.paragraph_format.first_line_indent = 0
        p.paragraph_format.line_spacing = 1.0
        p.paragraph_format.space_before = 0
        p.paragraph_format.space_after = 0
        name, _, inches = name.partition("@")
        width = Inches(float(inches)) if inches else FIG_WIDTH
        p.add_run().add_picture(f"{FIGS}/{name}", width=width)
        keep_with_next(p)
        para("Slika", caption)
    elif kind == "table":
        caption, header, rows, widths = item[1:]
        add_table(caption, header, TABLE1 if rows == "TABLE1" else rows, widths)

keep_with_next(para("Heading 1", "References"))
for ref in REFERENCES:
    para("references", ref)

# The template's own figures are no longer referenced; drop them, or they
# stay inside the file.
xml = d.element.xml
for rid, rel in list(d.part.rels.items()):
    if "image" in rel.reltype and f'"{rid}"' not in xml:
        d.part.drop_rel(rid) if hasattr(d.part, "drop_rel") else d.part.rels.pop(rid)

d.save(OUT)

# Document properties: the template's still say "Paper Title" / "IEEE".
# python-docx would add a second core.xml to this converted package, so the
# existing one is rewritten in place.
import re
import zipfile

with zipfile.ZipFile(OUT) as z:
    parts = {n: z.read(n) for n in z.namelist()}
core = parts["docProps/core.xml"].decode()
for tag, value in (("dc:title", TITLE), ("dc:creator", "Aleksa Zdravković"),
                   ("cp:lastModifiedBy", "Aleksa Zdravković"),
                   ("dc:subject", "IEEESTEC 2026"), ("cp:keywords", KEYWORDS)):
    pattern = rf"<{tag}(\s[^>]*)?>.*?</{tag}>|<{tag}\s*/>"
    element = f"<{tag}>{value}</{tag}>"
    if re.search(pattern, core, flags=re.S):
        core = re.sub(pattern, element, core, flags=re.S)
    else:
        core = core.replace("</cp:coreProperties>", element + "</cp:coreProperties>")
import datetime

now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
for tag in ("dcterms:created", "dcterms:modified"):
    core = re.sub(rf"(<{tag}[^>]*>).*?(</{tag}>)", rf"\g<1>{now}\g<2>", core)
core = re.sub(r"<cp:revision>\d+</cp:revision>", "<cp:revision>1</cp:revision>", core)
core = re.sub(r"<cp:lastPrinted>.*?</cp:lastPrinted>", "", core)
parts["docProps/core.xml"] = core.encode()
with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED) as z:
    for name, data in parts.items():
        z.writestr(name, data)
print("wrote", OUT)
