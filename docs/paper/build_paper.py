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
    "products that span software and hardware with AI coding agents. Agents "
    "now write each part of a product quickly, but the facts the parts share "
    "are still copied between them and drift apart. Fiducial declares every "
    "fact once, generates every artifact from it – code, translations, "
    "circuit board and enclosure – and checks every change, so agents work "
    "within guarantees instead of being trusted. In a test of 20 realistic "
    f"mistakes, the checks caught {_ALL} before they could be merged."
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
     "Building a product used to take a team: someone for the circuit board, "
     "someone for the firmware, others for the app, the website and the "
     "enclosure. AI coding agents such as Claude Code [1] and Codex [2] are "
     "changing that. An agent can write a firmware driver, a web page, a "
     "database migration or a script that models a case in minutes, so one "
     "person can now attempt a whole product."),
    ("p",
     "Building such products – first Outreach [3], an off-grid messaging "
     "system made of radios, gateways, an app and printed cases, and then "
     "the platform described here – has shown where the difficulty "
     "has moved. Writing each part is no longer the slow step; keeping the "
     "parts in agreement is. In Outreach, the radio’s firmware and the app disagreed on "
     "the layout of a message, and a printed case did not fit the board it "
     "was made for. Neither part was wrong on its own. The facts they shared "
     "had been copied by hand from one tool to another, one copy changed, and "
     "the mistake appeared only when the parts met (Fig. 1a)."),
    ("fig", "fig1-one-fact.png",
     "One fact used in four places: (a) copied by hand, one copy changes and "
     "nothing notices; (b) declared once, every copy is generated from it and "
     "checked."),
    ("p",
     "Agents do not remove this problem; they speed it up. An agent sees "
     "only the codebase it works in, so the agent writing "
     "the firmware does not know the board changed, and the one writing the "
     "web page does not know the firmware did. Its output is also "
     "probabilistic: usually right, but never guaranteed. More code, written "
     "faster, in more disconnected places, gives an inconsistent product "
     "sooner."),
    ("p",
     "What is missing is not a better tool for any one discipline, but one "
     "shared and checked context for all of them: a single place where a "
     "product’s facts live, from which every part is produced, and which "
     "refuses a change that makes the parts disagree (Fig. 1b). This paper "
     "presents "
     "Fiducial [4], an open-source platform built to provide it, compares "
     "it with existing approaches, and tests whether its checks catch the "
     "mistakes that people and agents actually make."),

    ("h1", "Existing Approaches"),
    ("p",
     "Several kinds of tool already address parts of this problem, and each "
     "falls short in the same way: it keeps one kind of fact, or one pair of disciplines, in "
     "agreement, but not the product as a whole (Table I)."),
    ("p",
     "Model-driven and model-based systems engineering keep a design in one "
     "central model and generate or check other views from it [5], [6]. They "
     "are the closest in spirit, but the model lives in a specialised "
     "graphical tool, the code is still written by hand beside it, and an "
     "agent cannot easily work in it. Engineering suites link neighbouring "
     "disciplines instead: "
     "Altium’s MCAD CoDesigner passes changes between a circuit board and its "
     "mechanical model [7]. The firmware, the app and the translations stay "
     "outside, and the link is between two tools, not from one source."),
    ("p",
     "Design-as-code tools write hardware as text: a circuit in atopile [8], "
     "a case as a build123d script [9]. This is what makes hardware writable "
     "by an agent, and Fiducial uses build123d. But each covers one "
     "discipline. Fiducial’s first design planned to author boards in "
     "atopile, and found that a circuit language does not cover the case, "
     "the layout or the cost, so the board’s facts would still be copied "
     "into each of them. Interface languages such as Protocol Buffers "
     "declare a message once and generate the code for both sides [10]. That "
     "removes the drift met in Outreach, but only for messages; a pin, a "
     "colour or a board size has no such home."),
    ("table", "Existing Approaches and Where They Stop",
     ("Approach", "Keeps in agreement", "Stops at"), [
        ("Model-based engineering", "views of one model",
         "graphical tool; code by hand"),
        ("ECAD–MCAD links", "board and case", "two disciplines, two tools"),
        ("Design as code", "one discipline", "facts shared with others"),
        ("Interface languages", "messages, both sides", "everything else"),
        ("Build systems", "outputs with files", "two files that disagree"),
        ("Agent context files", "nothing; they advise", "not enforced"),
        ("Fiducial", "every declared fact", "facts not yet declared"),
    ], (1500, 1550, 1500)),
    ("p",
     "Build systems such as Bazel [11] and Nix [12] rebuild exactly what "
     "changed, and Nix identifies every result by a fingerprint of its "
     "inputs. They guarantee that outputs follow from files, not that files "
     "agree with each other: the same fact typed into two source files "
     "builds without complaint. Coding agents, finally, follow context files "
     "such as AGENTS.md [13], which describe a repository’s rules but do not "
     "enforce them. A known way to use a generator that can be wrong is to "
     "pair it with a checker that cannot [14], but no existing checker spans "
     "a whole product."),
    ("p",
     "Fiducial keeps what works in each – text that agents can write, "
     "outputs tied to inputs by fingerprints, established engines for hard "
     "problems, and checks that decide – and changes the unit. It tracks "
     "facts rather than "
     "files or tools, across every discipline of one product, so any fact – "
     "a message, a pin, a colour, a size – has exactly one home."),

    ("h1", "The Approach"),
    ("p",
     "Fiducial treats a product as two kinds of thing. Facts are the "
     "decisions that define it: which pin a sensor is connected to, what a "
     "button says in each language, the brand colour, the size of the board, "
     "the layout of a message between a device and its app. Artifacts are "
     "everything built from those facts: source code, schematics, circuit "
     "boards, translation files, web pages, enclosure models. Drift is what "
     "happens when an artifact stops agreeing with a fact, or with another "
     "artifact that uses the same fact (Fig. 1a). The approach consists of "
     "three rules that together make drift visible and keep it out of the "
     "product (Fig. 1b)."),
    ("p",
     "Declare once. Every fact is written in exactly one place, a "
     "declaration: a typed text file in the product’s repository. Typed means "
     "the file has a fixed structure that a program can check, not free "
     "prose. A fact that exists once has no second copy to disagree with."),
    ("p",
     "Derive everything else. Every artifact that depends on a fact is "
     "generated from its declaration by one command, and generated files are "
     "never edited by hand. Generation is deterministic: the same "
     "declarations always produce exactly the same files, which makes the "
     "product reproducible and makes any difference meaningful."),
    ("p",
     "Check every change. Before a change is accepted, checks confirm that "
     "every artifact still matches what its declarations produce now, and "
     "that the declarations themselves are valid. A failing check names the "
     "declaration and the file at fault, so the person or agent knows what to "
     "fix."),
    ("p",
     "These rules change the role of an AI agent. The agent still edits "
     "declarations and code, but it is no longer trusted to keep anything "
     "consistent: generation does that, and the checks confirm it. Its "
     "mistakes are not prevented, but they are caught before they reach the "
     "product, with a message precise enough for the agent to correct "
     "itself. Hence the name: a fiducial is a fixed reference point, a "
     "universal measure that everything is aligned to and compared against, "
     "so it stays uniform. The declarations are that reference for a "
     "product."),

    ("h1", "Implementation"),
    ("p",
     "Fiducial is a command-line tool, fid, written in Rust. Its parts are "
     "described below; Fig. 2 shows how a change moves through them."),
    ("h2", "One Structure for Every Product"),
    ("p",
     "fid new creates a product repository with a fixed layout, and fid add "
     "enables capabilities. A capability packages one discipline: its "
     "declarations, the generator that turns them into artifacts, its checks "
     "and instructions for agents. There are 23, from web applications and "
     "translations to firmware and hardware. Because every product has the "
     "same structure, whoever knows one knows them all, and a new product "
     "starts from parts that already work. Adding translations, for example, "
     "brings the language files, their generator, the check that every "
     "language has every string, and the agent’s instructions. fid upgrade brings platform "
     "improvements into each product while keeping its own edits."),
    ("h2", "Declarations and Generation"),
    ("p",
     "Declarations are plain text files in version control, from the "
     "strings of each language to every part of a board with the net each "
     "pin connects to. Text was chosen over a graphical modelling tool "
     "because coding agents read and write it well, and every change then "
     "goes through ordinary code review. fid "
     "derive runs each capability’s generator, which reads the declarations "
     "and writes the artifacts, from a typed module of translations to a "
     "firmware pin map or a KiCad circuit board [15]. A lock file records a "
     "cryptographic hash – a short fingerprint of a file’s contents – for "
     "every input and output, so the platform knows exactly what each "
     "artifact was made from."),
    ("h2", "Checks"),
    ("p",
     "Checks run in continuous integration, the automatic build that runs on "
     "every proposed change, and a change is merged only when they pass. "
     "There are three kinds. Freshness: fid derive --check compares every "
     "recorded fingerprint with the repository and fails if a declaration "
     "changed without its artifacts being regenerated, or if a generated "
     "file was edited by hand. Rules: generation refuses declarations that "
     "break a rule, such as a language missing a string, two colours with too "
     "little contrast, parts that cost more than the declared limit, or a "
     "board that does not fit its case. Consumers: compilers and type "
     "checkers read the generated files, so code that still uses a renamed "
     "fact stops compiling."),
    ("fig", "fig2-loop.png",
     "How a change moves through Fiducial: declarations are edited, "
     "generation produces the artifacts with the help of established "
     "engines, and the checks either accept the change or return a message "
     "naming the declaration at fault."),
    ("h2", "Engines for Hard Problems"),
    ("p",
     "Some generation steps are real engineering problems: where to place "
     "the parts on a board, how to connect them, how to build a case around "
     "them. The first version placed parts with a hand-written search, and "
     "each new product added special rules until they began to interact. Now "
     "the platform only writes the problem down and hands it to an "
     "established engine. A constraint solver, CP-SAT [16], is given the "
     "rules a placement must satisfy and finds one that does, or proves none "
     "exists; build123d [9] builds the case; KiCad routes and checks the "
     "board. Every placement rule is one of eight general kinds, such as "
     "inside, apart or near, so a new product needs new declarations rather "
     "than new code. The solver’s answer is stored with the fingerprint of "
     "the problem it solved, so checking a product never runs it again. The "
     "engines check as well as build: KiCad confirms the board can be made, "
     "and the case model confirms that the board fits and the lid closes."),
    ("h2", "Agents in the Loop"),
    ("p",
     "Every product carries an AGENTS.md file and one instruction file per "
     "capability, so any agent starts from the same description of the "
     "product, and a hook stops it from editing a generated file. fid dash "
     "shows the whole product’s state in one view, recomputed from the "
     "repository, so an agent starts from facts rather than guesses. Agents fix "
     "a problem where the fact lives, never in the output. The platform grows "
     "the same way: a mistake that gets past the checks becomes a general "
     "rule for every product."),

    ("h1", "Example: A Sensor Stick"),
    ("p",
     "The repository contains a complete example product: a USB-C stick that "
     "measures temperature and humidity and shows them on a web page "
     "(Fig. 3). It is small, but it has every discipline: a printed case, a "
     "circuit board, firmware in Rust and a page that reads the sensor in "
     "the browser. It was chosen as the smallest "
     "product that still crosses all of them, so every rule described above "
     "applies to it."),
    ("fig", "fig3-sensor-stick.png",
     "The sensor stick, generated from its declaration: (a) the closed case; "
     "(b) the routed board inside it, with the sensor under its vent."),
    ("p",
     "The person writes the product declaration, an outline of its shape, "
     "the firmware’s logic and the page. Everything else is generated: the "
     "routed board with its schematic and bill of materials, the case built "
     "around it, the files a manufacturer needs, and the code the firmware "
     "and the page use to exchange readings. Moving the sensor to another "
     "pin is one changed line. Until the product is regenerated the check "
     "fails; afterwards the board, the firmware’s pin map and the page’s "
     "types have all followed, with no edit to the firmware or the page. If "
     "the new pin is one the chip’s sensor bus cannot use, the firmware stops "
     "compiling – before any board is made."),

    ("h1", "Evaluation"),
    ("p",
     "Fiducial was built over six weeks by one person working with AI "
     "agents, through 68 reviewed pull requests, and used on two products: "
     "the sensor stick and a private product. On the sensor stick, moving one "
     f"pin regenerates {_PIN_N} files in {_PIN_S} s."),
    ("p",
     "Checks are only useful if they catch the mistakes that people and "
     "agents actually make. The test borrows the idea of mutation testing "
     "[17]: a realistic mistake is introduced on purpose, the checks are "
     "run, and the result shows whether they noticed. Twenty mistakes cover "
     "eight kinds of drift met while building Fiducial and its products "
     "(Table II): ten in a fresh web product in English and Serbian, and ten "
     "in the sensor stick’s hardware, firmware and the interface between "
     "them. Each was applied to a clean copy, the checks ran as in "
     "continuous integration, and the copy was reset. Three correct changes "
     "and two runs with no change tested the opposite direction: that the "
     "checks do not fail without a reason. The test is a script in the "
     "repository, and every failure names the file and the declaration at "
     "fault (Fig. 4)."),
    ("p", "FINDINGS_A"),
    ("table", "Injected Mistakes, by Kind of Drift",
     ("Kind of drift", "Example", "0.9.4", VERSION), "TABLE1",
     (1330, 1780, 640, 760)),
    ("fig", "fig4-failures.png",
     "Two failures as the person or agent sees them: (a) a string added in "
     "English only; (b) firmware naming a pin directly instead of taking it "
     "from the declaration."),
    ("p", "FINDINGS_B"),
    ("p",
     "The mistakes were chosen by the author of the platform, so they may "
     "favour what it was built to catch. Only two products have been built, "
     "and neither has yet been "
     "manufactured and tested. The test measures the checks, not whether "
     "agents working with Fiducial make fewer mistakes than agents working "
     "without it."),

    ("h1", "Conclusion"),
    ("p",
     "AI agents have made writing each part of a product fast, and that has "
     "moved the hard problem to keeping the parts in agreement. Better "
     "models will move the line between what a person checks and what is "
     "left to agents, but a probabilistic system cannot guarantee its "
     "output, and even a deterministic tool is only right about what it was "
     "told. Fiducial answers this at the level of the product rather than "
     "the codebase. In practice, it let one person working with AI agents keep "
     "a product’s firmware, web page, circuit board and case in agreement, "
     "with every injected mistake caught early by a message that names its "
     "cause. The next steps are to manufacture the sensor stick and to "
     "compare agents working with and without the platform. Fiducial and its "
     "example are open source [4]."),
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

FINDINGS_A = (
    "On release 0.9.4 the checks caught 15 of the 20, and each of the five "
    "misses was a fact the platform did not yet own. Two were declarations "
    "edited without regenerating: only the hardware generator recorded which "
    "files it read, so the freshness check did not notice that the others’ "
    "inputs had changed. In two, the firmware wrote a pin number or a sensor "
    "address directly instead of taking it from the declaration. The last "
    "was a Serbian string left in English, complete in structure but not "
    "translated."
)
FINDINGS_B = (
    "Each miss became a general rule: every generator now records its "
    "inputs, firmware that writes a declared value directly is refused, and "
    "a translation identical to the default language fails unless it is "
    f"declared the same on purpose. On release {VERSION}, "
    + ("all 20 are caught" if ALL_CAUGHT else f"{_N} of the 20 are caught")
    + f" – {_count('derive')} during generation, {_count('--check')} by the "
    f"freshness check and {_N - _count('derive') - _count('--check')} by the "
    "compiler – "
    + ("and every correct change passes."
       if CONTROLS_OK else
       f"and {_SUM['controls_passed']} of the {_SUM['controls']} controls pass.")
)

REFERENCES = [
    "Claude Code documentation. Available at: "
    "https://docs.anthropic.com/en/docs/claude-code",
    "OpenAI Codex. Available at: https://openai.com/codex/",
    "A. Zdravković, Đ. Jovanović and P. Šubarević, “Outreach: Modular "
    "AI-enhanced off-grid mesh communication system for remote "
    "environments,” IEEESTEC – 18th Student Projects Conference, Niš, 2025.",
    f"A. Zdravković, Fiducial, release {VERSION}, 2026. Available at: "
    "https://github.com/AleksaZCodes/fiducial",
    "D. C. Schmidt, “Guest editor’s introduction: Model-driven "
    "engineering,” Computer, vol. 39, no. 2, pp. 25–31, 2006.",
    "S. J. I. Herzig, A. Qamar and C. J. J. Paredis, “An approach to "
    "identifying inconsistencies in model-based systems engineering,” "
    "Procedia Computer Science, vol. 28, pp. 354–362, 2014.",
    "Altium, MCAD CoDesigner, ECAD–MCAD collaboration. Available at: "
    "https://www.altium.com/",
    "atopile, a language for designing circuit boards as code. Available "
    "at: https://github.com/atopile/atopile",
    "build123d, a Python CAD library on the OpenCascade kernel. Available "
    "at: https://github.com/gumyr/build123d",
    "Protocol Buffers. Available at: https://protobuf.dev/",
    "Bazel, a build and test tool. Available at: https://bazel.build/",
    "E. Dolstra, M. de Jonge and E. Visser, “Nix: A safe and policy-free "
    "system for software deployment,” in Proc. LISA, 2004, pp. 79–92.",
    "AGENTS.md, a format for guiding coding agents. Available at: "
    "https://agents.md/",
    "S. Kambhampati et al., “Position: LLMs can’t plan, but can help "
    "planning in LLM-Modulo frameworks,” in Proc. ICML, PMLR 235, 2024.",
    "KiCad EDA. Available at: https://www.kicad.org/",
    "L. Perron and F. Didier, CP-SAT, Google OR-Tools. Available at: "
    "https://developers.google.com/optimization/cp/cp_solver",
    "R. A. DeMillo, R. J. Lipton and F. G. Sayward, “Hints on test data "
    "selection: Help for the practicing programmer,” Computer, vol. 11, "
    "no. 4, pp. 34–41, 1978.",
]

FIG_WIDTH = Inches(3.3)  # the column is 3.38 in; figure labels are 8 pt at this width

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


# The title section came out of the .doc conversion with a 1872-twip right
# margin. Word lays the title out across the full width regardless (the
# Outreach paper's Word PDF centres it on the page), but LibreOffice takes
# the first section's margins as the page's and narrowed every column to
# about 3.05 in, so its preview ran longer than Word's print. Giving the
# title section the body's margins makes both agree with the Word PDF:
# two 3.38 in columns from 0.63 in to 7.64 in.
_BODY_MARGIN = docx.shared.Twips(907)
d.sections[0].left_margin = _BODY_MARGIN
d.sections[0].right_margin = _BODY_MARGIN

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
        para("Body Text", {"FINDINGS_A": FINDINGS_A,
                           "FINDINGS_B": FINDINGS_B}.get(item[1], item[1]))
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
        # The caption ends the figure: it must not pull the heading or
        # paragraph after it along, or a page break leaves a gap.
        para("figure caption", caption).paragraph_format.keep_with_next = False
    elif kind == "table":
        caption, header, rows, widths = item[1:]
        add_table(caption, header, TABLE1 if rows == "TABLE1" else rows, widths)

# References is a component head: Heading 5, unnumbered, as the template says.
keep_with_next(para("Heading 5", "References"))
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

# The .doc → .docx conversion switched automatic hyphenation on. Word did not
# hyphenate the Outreach paper it printed from the same template, and the
# LibreOffice preview cannot (no dictionary), so with it on, Word would break
# lines differently from the preview every layout decision was checked on.
settings = parts["word/settings.xml"].decode()
settings = re.sub(r'<w:autoHyphenation w:val="(true|1)"/>', '', settings)
parts["word/settings.xml"] = settings.encode()
with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED) as z:
    for name, data in parts.items():
        z.writestr(name, data)
print("wrote", OUT)
