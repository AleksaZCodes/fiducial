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

_WORDS = {1: "one", 2: "two", 3: "three", 4: "four", 5: "five", 6: "six", 7: "seven",
          8: "eight", 9: "nine", 10: "ten", 20: "twenty"}


def _count(gate):
    """How many markers a gate caught first."""
    return sum(1 for r in _RESULTS["results"]
               if not r["control"] and r["caught_by"] and r["caught_by"][0] == gate)


_GATE = {"derive": "derive", "--check": "--check", "firmware build": "compiler",
         "firmware tests": "firmware test", "web test": "page test", "type check": "type check"}


def _now(mid):
    """The first gate that caught it: once derive fails, --check fails too,
    so naming the second says nothing new."""
    by = [_GATE[g] for g in _BY_ID[mid]["caught_by"] if g in _GATE]
    return by[0] if by else "not caught"


_C1 = _BY_ID["C1"]
PINMOVE = (f"{len([f for f in _C1['changed'] if '/generated/' in f])} generated files "
           f"rewritten in {round(next(g['seconds'] for g in _C1['gates'] if g['gate'] == 'derive'))} s")

ALL_CAUGHT = _SUM["markers_caught"] == _SUM["markers"]
CONTROLS_OK = _SUM["controls_passed"] == _SUM["controls"]

ABSTRACT = (
    "This paper presents Fiducial, an open-source platform for building "
    "products that span software and hardware – a web application, firmware, "
    "a circuit board and a printed case – with AI coding agents. It gives "
    "every product the same structure: each fact is declared once in typed "
    "files, every artifact that depends on it is derived by one command, and "
    "checks on every commit refuse anything that no longer follows from its "
    "declaration. Hard problems such as part placement and solid modelling "
    "are handed to established engines, so the platform grows by "
    "declarations rather than by special cases. To test it, 20 realistic "
    "mistakes were injected into two products. The first run caught 15. Each "
    "miss was turned into a general rule, and the current version catches "
    f"{_SUM['markers_caught']} of the 20 while letting every correct change "
    "through."
)

KEYWORDS = (
    "AI agents, single source of truth, code generation, consistency "
    "checking, hardware–software co-design, continuous integration"
)

_first = _count("derive")
_check = _count("--check")
_consumers = _SUM["markers_caught"] - _first - _check

# ("h1", text) | ("h2", text) | ("p", text) | ("fig", file[@inches], caption)
# | ("table", caption, header, rows, widths) — a row that is a str is a group
BODY = [
    ("h1", "Introduction"),
    ("p",
     "While building Outreach [1], an off-grid messaging system made of "
     "radios, gateways, a companion app and printed cases, the author ran "
     "into the same problem more than once. The firmware and the app "
     "disagreed on the layout of a message, and a printed case did not fit "
     "the board it was made for. Neither part was wrong on its own. The facts "
     "they shared were copied by hand between different tools, one copy "
     "changed, and the mistake showed up only when the parts were put "
     "together."),
    ("p",
     "This is how most products are built, and AI coding agents [2], [3] make "
     "it faster without making it safer. An agent is quick inside one "
     "codebase but knows nothing about the others: the agent writing the "
     "firmware does not know the board changed, and the one writing the web "
     "page does not know the firmware did. Every project also invents its own "
     "layout, scripts and habits, so each new product starts from scratch and "
     "every agent has to learn it again."),
    ("p",
     "Fiducial [4] takes the opposite approach. Every product is built the "
     "same way, in one repository, around one rule: each fact is declared "
     "once, and everything that needs it is derived from it and checked "
     "(Fig. 1). People and agents edit the declarations, never the outputs. "
     "This paper explains the idea, how the platform is put together and "
     "why, and how its checks hold up against mistakes made on purpose."),
    ("fig", "fig1-one-fact.png",
     "One fact in four places. (a) Copied by hand: one copy changes and "
     "nothing notices. (b) Declared once: every copy is derived from it and "
     "checked on every commit."),

    ("h1", "The Idea"),
    ("p",
     "A product is a set of facts – which pin a sensor is on, what a button "
     "says in each language, the brand colour, the size of the board – and a "
     "set of artifacts that use them: code, schematics, translations, web "
     "pages, case models. Most bugs between disciplines come from one fact "
     "being written in more than one artifact. Fiducial removes that with "
     "three rules."),
    ("p",
     "Declare once. Every fact has exactly one place where it is written, a "
     "typed text file that a person or an agent edits."),
    ("p",
     "Derive everything else. One deterministic command, fid derive, "
     "generates every artifact that depends on a fact. Generated files are "
     "never edited by hand."),
    ("p",
     "Check every change. On every commit, checks confirm that every artifact "
     "still follows from its declarations and that the declarations make "
     "sense. A failure names the declaration at fault."),
    ("p",
     "None of these rules is new on its own. Model-driven engineering "
     "generates code from models [5], and Nix identifies every build by a "
     "hash of its inputs [6]. What Fiducial adds is applying them across "
     "every discipline of one product at once, in plain files that coding "
     "agents read and write well, with the checks – not the agent’s good "
     "behaviour – as what makes the result reliable."),

    ("h1", "How It Is Built"),
    ("h2", "One Structure for Every Product"),
    ("p",
     "fid new creates a product repository with a fixed layout, and fid add "
     "turns on capabilities: packaged pipelines for web frameworks, "
     "translations, content, search metadata, legal pages, design systems, "
     "databases, deployment, device messages, firmware for two microcontroller "
     "families, and hardware – 23 in all. Two products built with Fiducial look the same, "
     "so knowing one means knowing the others, and a new product starts from "
     "working, checked parts instead of an empty folder. Every product also "
     "carries an AGENTS.md file [7] and one instruction file per capability, "
     "so any agent starts from the same context. A newer platform version "
     "comes in with one command, fid upgrade, which keeps the product’s own "
     "edits."),
    ("h2", "Declarations"),
    ("p",
     "Declarations are typed text files in version control: messages/"
     "<language>.json for every user-visible string, fiducial.toml for pages "
     "and brand, design-system.md for colours and the contrast they must "
     "reach, protocol.toml for the messages a device sends, and "
     "hardware/product.toml for the parts of a physical product and how they "
     "connect. Plain text was chosen over a modelling tool because it is what "
     "coding agents handle best, and because every change then goes through "
     "a normal code review. A declaration states a fact and nothing more. It "
     "does not say where the sitemap lists a page or which way a trace runs."),
    ("h2", "Derivation"),
    ("p",
     "fid derive runs every enabled pipeline. Each one reads the declarations "
     "it needs and writes its artifacts: a typed message module, a sitemap, a "
     "database migration, a firmware pin map, TypeScript types, a circuit "
     "board. Every pipeline lists the files it reads and writes, and the "
     "SHA-256 hash of each is stored in a lock file. Derivation is "
     "deterministic, so the same inputs always give the same bytes; without "
     "that, comparing hashes would mean nothing. This requirement decided "
     "more than one design choice: the placement solver, for example, runs "
     "on a single thread, because with several it gave different answers to "
     "the same problem."),
    ("h2", "Checks"),
    ("p",
     "The checks are of three kinds (Fig. 2). Freshness: fid derive --check "
     "fails when a declaration changed without its outputs being regenerated, "
     "or when an output was edited by hand. Rules inside derivation: every "
     "language must have every string, every page needs a title, every colour "
     "pair must reach its contrast, circuit voltages must fall inside their "
     "windows, the parts must cost less than a declared limit, and the board "
     "must fit its case. Consumers: compilers and type checkers read the "
     "derived files, so code that still uses a renamed fact stops compiling. "
     "All of them run in continuous integration on every commit. A failure "
     "names the file and the declaration at fault, which is the kind of "
     "signal an agent needs to fix its own mistake."),
    ("fig", "fig2-loop.png@2.9",
     "How a change moves through Fiducial. Declarations are edited, "
     "derivation produces the artifacts and hands hard problems to "
     "established engines, and the checks either let the change through or "
     "return a message naming the declaration at fault."),
    ("h2", "Hard Problems Go to Engines"),
    ("p",
     "Some derivations are real engineering problems: where to put the parts "
     "on a board, how to route it, how to build a case around it. The first "
     "version placed parts with a hand-written search, and every new product "
     "added another special rule. The rules began to interact, and the code "
     "grew faster than the products. So the structure changed: the platform "
     "writes the problem down in a fixed format and hands it to an "
     "established engine – the CP-SAT constraint solver [8] for placement, "
     "the OpenCascade kernel through build123d [9] for the case, and KiCad "
     "[10] for the board and its design-rule check. Every placement rule "
     "became one of eight generic kinds, and a new requirement became a new "
     "declaration instead of new code. When declarations contradict each "
     "other, the solver names the smallest set that cannot all hold."),
    ("h2", "Mistakes Become Rules"),
    ("p",
     "Two habits keep the platform growing in the right direction. Agents "
     "fix a problem in the layer that owns the fact – a wrong pin in the "
     "declaration, a wrong derivation in the platform – and never in a "
     "generated file. And when a mistake gets past the checks, it is not "
     "patched for that one case; it becomes a general rule that holds for "
     "every product. Section V shows what that looks like."),

    ("h1", "Example: A Sensor Stick"),
    ("p",
     "To show the whole chain on something anyone can inspect, the "
     "repository contains a public example: a USB-C stick that reports "
     "temperature and humidity to a web page (Fig. 3). It is small on "
     "purpose, but it has every discipline: a printed case, a routed "
     "two-layer board with 23 parts, Rust firmware and a web page. The person "
     "writes the product declaration, an outline drawing, the firmware and "
     "the page. Fiducial derives the rest, including the files a board house "
     "needs to make it."),
    ("fig", "fig3-sensor-stick.png",
     "The sensor stick, derived from its declaration: (a) the closed case, "
     "36 × 84 mm; (b) the routed board inside it, with the sensor at the far "
     "end under its vent."),
    ("p",
     "The message the stick sends is declared once as well, and both the "
     "firmware’s encoder and the page’s decoder are generated from it. That "
     "closes the exact gap met in Outreach, where the two sides were written "
     "separately. Moving the sensor’s data line to another pin is one "
     "changed line. Until the product is re-derived, the check fails; after "
     "fid derive, the board, the schematic, the firmware’s pin map and the "
     "page’s types all follow, and the firmware and the page need no edit. If "
     "the new pin is one the chip’s I²C bus cannot use, the firmware stops "
     "compiling, because in Embassy [11] every pin is its own type."),

    ("h1", "Evaluation"),
    ("h2", "Development"),
    ("p",
     "Fiducial was built over six weeks, from 6 September to 15 October 2026, "
     "by one person working with AI agents: about 240 commits and 68 merged "
     "pull requests, with about 65,000 lines of Rust, 9,000 of TypeScript and "
     "6,000 of Python. It has been used on two products, the sensor stick and "
     "a private one. Table I lists measurements from both."),
    ("table", "Measured Results", ("Measure", "Result"), [
        ("Placement rule kinds", "8; none added for either product"),
        ("Sensor stick board", "26 × 79 mm, 2 layers, 0 DRC errors"),
        ("Sensor stick parts", "USD 4.74 per board at 5 (limit USD 15)"),
        ("One pin moved", PINMOVE),
        ("Placement, private product",
         "13 % shorter connections than the hand-written search"),
        ("Contradictions", "named as the smallest conflicting set"),
    ], (1700, 3080)),
    ("h2", "Injected Mistakes"),
    ("p",
     "Checks are only worth having if they catch the mistakes people "
     "actually make. To measure that, the test follows the idea of mutation "
     "testing [12]: inject a realistic mistake, run the checks, and see "
     "whether they notice. Twenty mistakes were chosen to cover the eight "
     "kinds of drift met while building Fiducial and its products (Table "
     "II): ten in a fresh web product in English and Serbian with ten "
     "software capabilities, and ten in the sensor stick’s hardware, firmware "
     "and interface. Each is applied to a clean copy, the checks run in the "
     "same order as in CI – derive, --check, the firmware build and tests, "
     "the page’s test and type check – and the copy is reset. Three correct "
     "changes and two runs with no change test the other direction: that the "
     "checks do not fail without a reason. The test is a script in the "
     "repository and runs with one command."),
    ("p",
     "On release 0.9.4 the checks caught 15 of the 20. The five misses were "
     "not random: each was a fact the platform did not yet own. Two "
     "declarations edited without re-deriving went unnoticed, because only "
     "the hardware pipeline listed the files it read. Firmware that named a "
     "pin by its number kept compiling after the pin moved. A mistyped sensor "
     "address existed only in the firmware. A Serbian string left in English "
     "passed every structural check. And the firmware and the page "
     "disagreeing on a message layout was caught only by a hand-written "
     "test."),
    ("table", "Injected Mistakes, by Kind of Drift", ("Mistake", "0.9.4", "Now"), "TABLE2",
     (2560, 1100, 1100)),
    ("p", "SECONDRUN"),
    ("fig", "fig4-failures.png",
     "Two failures from the test, as the person or agent sees them: (a) a "
     "string added in English only; (b) firmware that names a pin directly "
     "instead of taking it from the declaration."),
    ("p",
     "The twenty were written by the author, who also wrote the platform, "
     "so they may favour what it was built to catch. Later rounds of "
     "mistakes, written separately from the platform’s code, found further "
     "gaps – among them a unit mismatch between firmware and page, an I²C "
     "line with no pull-up resistor, and cookie consent that started with a "
     "choice already made – and each was again turned into a rule."),

    ("h1", "Discussion"),
    ("p",
     "Better models move the line between what a person must check and what "
     "can be left to an agent, but they do not remove it. A probabilistic "
     "system cannot be guaranteed to be correct, and even a deterministic "
     "tool is only right about what it was told. Fiducial bounds that "
     "uncertainty: declarations are the constraints, derivation is "
     "repeatable, and the checks refuse drift. The person still judges what "
     "no check can. During this work that meant a parts catalogue that "
     "changed format overnight, a text match that read “±0.1 %” as “1 %”, "
     "and a sensor that had to be moved away from the board’s heat."),
    ("p",
     "The main lesson is about structure more than any one tool. Keeping "
     "every discipline in one repository with one shape made the product "
     "something an agent can see whole, and turning each escaped mistake "
     "into a rule made the checks stronger with use."),
    ("p",
     "The evidence has clear limits. Only two products were built, one of "
     "them public, and neither has been manufactured and tested yet. The test "
     "measures the checks, not whether agents working with Fiducial make "
     "fewer mistakes than agents working without it. Circuit checks are DC "
     "only, and autorouting is not deterministic, so a routed board is "
     "checked rather than compared byte for byte."),

    ("h1", "Conclusion"),
    ("p",
     "Fiducial gives a product one structure across software and hardware: "
     "each fact declared once, everything else derived, and every change "
     "checked. That makes development quicker, because a new product starts "
     "from working parts and a change is made in one place, and more "
     "reliable, because a mistake is caught with a message that names its "
     "cause. Twenty injected mistakes found five gaps in the first run; once "
     "each became a general rule, "
     + ("all twenty are caught" if ALL_CAUGHT
        else f"{_SUM['markers_caught']} of the twenty are caught")
     + ". The next steps are to manufacture the sensor stick and to compare "
     "agents working with and without the platform on the same tasks. The "
     "platform and the example are open source [4]."),
]

# Table II: the twenty, grouped by the kind of drift each one tests, with
# what caught it on 0.9.4 (the first run) and on this version.
_GROUPS = [
    ("Stale output", [("W2", "English string edited, not re-derived", "not caught"),
                      ("W6", "Domain changed, not re-derived", "not caught"),
                      ("H1", "Sensor pin moved, not re-derived", "--check")]),
    ("Generated file edited", [("W3", "Message module edited", "--check"),
                               ("W7", "Worker config edited", "--check"),
                               ("W9", "Applied migration edited", "--check"),
                               ("H2", "Firmware pin map edited", "--check"),
                               ("H8", "Message layout changed in firmware", "test only")]),
    ("Incomplete declaration", [("W1", "String added in English only", "derive"),
                                ("W4", "Post published in English only", "derive"),
                                ("W5", "Page without a title", "derive")]),
    ("Declaration breaks a rule", [("W8", "Text below 4.5 : 1 contrast", "derive"),
                                   ("H3", "USB-C pull-down 5.1 → 22 kΩ", "derive"),
                                   ("H5", "Cost limit lowered to USD 3", "derive")]),
    ("Declarations conflict", [("H4", "Sensor zone contradicts its vent", "derive"),
                               ("H10", "Case narrowed 36 → 28 mm", "derive")]),
    ("Code out of date", [("H6", "Net renamed, code not updated", "compiler")]),
    ("Declaration bypassed", [("H7", "Pin named by number, then moved", "not caught"),
                              ("H9", "Sensor address typed in firmware", "not caught")]),
    ("Copied, not translated", [("W10", "Serbian string left in English", "not caught")]),
]
TABLE2 = []
for group, rows in _GROUPS:
    TABLE2.append(group)
    for mid, mistake, first in rows:
        TABLE2.append((mistake, first, _now(mid)))

_c1 = _BY_ID.get("C1")
_c1_derive = next((g["seconds"] for g in _c1["gates"] if g["gate"] == "derive"), None) if _c1 else None
SECONDRUN = (
    "Each miss became a general rule: every pipeline lists its inputs; "
    "firmware that names a pin or an I²C address directly is refused; the "
    "sensor’s address is declared with the part and derived into the "
    "firmware; a translation identical to the default language fails unless "
    "it is declared the same on purpose; and every message is declared once, "
    "with both sides’ code generated. On the current version, "
    + (f"all {_WORDS.get(_SUM['markers'], _SUM['markers'])} mistakes are caught"
       if ALL_CAUGHT else f"{_SUM['markers_caught']} of the {_SUM['markers']} mistakes are caught")
    + f": {_WORDS.get(_first, _first)} during derivation, {_WORDS.get(_check, _check)} by "
    f"--check and {_WORDS.get(_consumers, _consumers)} by the compiler. "
    + ("Every correct change passes and the runs with no change leave every file as it was. "
       if CONTROLS_OK else
       f"{_SUM['controls_passed']} of the {_SUM['controls']} correct or unchanged runs pass. ")
    + "Every failure names the file and the declaration at fault (Fig. 4)."
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
    "AGENTS.md, a format for guiding coding agents. Available at: "
    "https://agents.md/",
    "L. Perron and F. Didier, CP-SAT, Google OR-Tools. Available at: "
    "https://developers.google.com/optimization/cp/cp_solver",
    "build123d, a Python CAD library on the OpenCascade kernel. Available "
    "at: https://github.com/gumyr/build123d",
    "KiCad EDA. Available at: https://www.kicad.org/",
    "Embassy, an async framework for embedded Rust. Available at: "
    "https://embassy.dev/",
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
        para("Body Text", SECONDRUN if item[1] == "SECONDRUN" else item[1])
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
        add_table(caption, header, TABLE2 if rows == "TABLE2" else rows, widths)

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
