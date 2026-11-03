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

TITLE = "Fiducial: Keeping AI-Built Products Consistent"

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
# The release the results were measured on, which must be the one cited.
RES_VERSION = _SUM["fid"].removeprefix("fid ")
assert RES_VERSION == VERSION, \
    f"the results are from {RES_VERSION} but the release is {VERSION}: rerun the sets"
# How many capabilities the platform ships, counted rather than remembered.
_CAPS = len([d for d in (_HERE.parents[1] / "crates/fiducial-cli/capabilities").iterdir()
             if d.is_dir()])

def _count(gate):
    """How many markers a gate caught first."""
    return sum(1 for r in _RESULTS["results"]
               if not r["control"] and r["caught_by"] and r["caught_by"][0] == gate)


_C1 = _BY_ID["C1"]

def _load(name):
    return _json.loads((_HERE / "markers" / name).read_text())


_HELD = _load("results-heldout.json")
_IND = _load("results-independent.json")
_FOURTH = _load("results-fourth.json")
_BASE = _load("results-baseline.json")
_HBASE = _load("results-heldout-baseline.json")
_IBASE = _load("results-independent-baseline.json")
_FBASE = _load("results-fourth-baseline.json")
_FIRST = _load("first_runs.json")
_H = _HELD["summary"]["markers_caught"]
_I = _IND["summary"]["markers_caught"]
_F = _FOURTH["summary"]["markers_caught"]
_HB = _HBASE["summary"]["markers_caught"]
_IB = _IBASE["summary"]["markers_caught"]
_FB = _FBASE["summary"]["markers_caught"]
_B = _BASE["summary"]["markers_caught"]
_UNSEEN_CI = _HB + _IB + _FB
_F1 = {k: _FIRST[k]["caught"] for k in ("original", "heldout", "independent", "fourth")}
# The text names what is still missed; a rerun that changes it must change it.
assert {r["id"] for r in _FOURTH["results"] if not r["pass"]} == {"F17"}, \
    "the fourth set's misses changed: update FINDINGS"
assert _H == _I == 20, "an earlier set regressed: update FINDINGS"
assert _UNSEEN_CI == 0, "ordinary CI caught an unseen mistake: update FINDINGS"

ALL_CAUGHT = _SUM["markers_caught"] == _SUM["markers"]
CONTROLS_OK = _SUM["controls_passed"] == _SUM["controls"]

_PIN_S = round(next(g["seconds"] for g in _C1["gates"] if g["gate"] == "derive"))
_PIN_N = len([f for f in _C1["changed"] if "/generated/" in f])
_N = _SUM["markers_caught"]
_ALL = "all 20" if ALL_CAUGHT else f"{_N} of the 20"

import math as _math
_POOL = _F1["heldout"] + _F1["independent"] + _F1["fourth"]
_z = 1.96
_c = (_POOL / 60 + _z**2 / 120) / (1 + _z**2 / 60)
_h = _z * _math.sqrt(_POOL / 60 * (1 - _POOL / 60) / 60 + _z**2 / 14400) / (1 + _z**2 / 60)
_LO, _HI = round(100 * (_c - _h)), round(100 * (_c + _h))
_PCT = round(100 * _POOL / 60)
_AFTER = _H + _I + _F
_MISSES = 60 - _POOL

ABSTRACT = (
    "This paper presents Fiducial, an open-source platform for building "
    "products that span software and hardware with artificial intelligence "
    "(AI) coding agents. Agents now write each part of a product quickly, "
    "but the facts the parts share are still copied between them and drift "
    "apart. Fiducial keeps each declared fact in one place, generates every "
    "artifact from it (code, translations, circuit board and enclosure) and "
    "checks every change, so the parts stay consistent with each other; "
    "whether a declared fact is itself correct remains for the person to "
    "judge. On 60 realistic mistakes in three sets, written by AI agents and "
    "saved before they were run, the checks caught "
    f"{_POOL} ({_PCT}%; 95% interval {_LO}–{_HI}%) on first run, where the "
    "same projects’ ordinary continuous integration (CI) caught none. All "
    f"but one of the {60 - _POOL} misses fell into one of five classes of "
    "mistake; after one fix for each class, the checks catch "
    f"{_AFTER} of the same 60, a measure of fit, not of generalization."
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
     "someone for the firmware, others for the app and the enclosure. Artificial intelligence (AI) "
     "coding agents such as Claude Code [@claude] and Codex [@codex] are "
     "changing that: a firmware driver, a web page or a script that models a "
     "case can be written in minutes, so one person can now attempt a whole "
     "product."),
    ("p",
     "Such a product is more than code. It has a circuit board and an "
     "enclosure, firmware, and an app or web page. It has text in every "
     "language it supports (translations, also called internationalization), "
     "a visual identity of colors, fonts and a logo, and a design system: the "
     "shared rules that keep its look the same and readable everywhere. "
     "Around these sit legal pages, search-engine metadata, a database "
     "schema and deployment settings. These parts look separate, but they "
     "share facts: a sensor reading’s scale is in the firmware, the web page "
     "and the database, a brand color is in the page, the logo and the case, "
     "and a label must exist in every language."),
    ("p",
     "This paper argues that there is a real advantage in centralizing all "
     "of this: the knowledge, the design system and the conventions in one "
     "place, and the work in one monorepository, a single repository that "
     "holds the code of many projects, as at Google [@monorepo]. Kept in a "
     "structured way, it also becomes reusable: a brand declared once supplies its colors, logo and "
     "3D renderings wherever needed. Fiducial is built on this idea and "
     "is designed to cover all of the above."),
    ("p",
     "Building such products, first Outreach [@outreach], an off-grid "
     "messaging system of radios, gateways, an app and printed cases, and "
     "then the platform described here, has shown where the difficulty has "
     "moved. Writing each part is no longer the slow step; keeping the parts "
     "in agreement is. In Outreach [@outreach], the firmware and the app disagreed on "
     "the layout of a message, and a printed case did not fit its board. "
     "Neither part was wrong on its own: a shared fact had been copied by "
     "hand, one copy changed, and the mistake appeared only when the parts "
     "met (Fig. 1a)."),
    ("fig", "fig1-one-fact.png",
     "One fact, the scale of a sensor reading, used in four places: (a) copied "
     "by hand, one copy changes and nothing notices; (b) declared once, every "
     "copy is generated from it and checked."),
    ("p",
     "Agents make this problem faster. An agent sees only the code it works "
     "in: the one writing the firmware does not know the board changed, and "
     "the one writing the web page does not know the firmware did. Its output "
     "is usually right but never guaranteed, so more code written faster in "
     "more places gives an inconsistent product sooner."),
    ("p",
     "What is missing is one shared and checked context for every "
     "discipline: a single place where a product’s facts live, from which "
     "every part is produced, and which refuses a change that makes the parts "
     "disagree (Fig. 1b). This paper presents Fiducial [@fiducial], an "
     "open-source platform built to provide it, compares it with existing "
     "approaches and with ordinary continuous integration (CI), the automatic "
     "build that checks every proposed change, and tests its checks on mistakes they "
     "were not built for."),

    ("h1", "Existing Approaches"),
    ("p",
     "Several kinds of tool address parts of this problem; each keeps one "
     "kind of fact, or one pair of disciplines, in agreement (Table I)."),
    ("p",
     "Model-driven and model-based systems engineering keep a design in one "
     "central model and generate or check other views from it [@schmidt], "
     "[@brambilla], in languages such as the Systems Modeling Language (SysML) [@sysml], with methods for "
     "managing inconsistencies between views [@nuseibeh], [@herzig]. They "
     "are the closest in spirit, but the model lives in a specialized "
     "graphical tool, code is still written by hand beside it, and an agent "
     "cannot easily work in it. Engineering suites link two tools, such as a "
     "circuit board and its mechanical model [@altium]; the firmware, app "
     "and translations stay outside."),
    ("p",
     "Design-as-code tools write hardware as text (a circuit in atopile "
     "[@atopile], a case in build123d [@build123d]), which makes it writable "
     "by an agent, but each covers one discipline. Interface languages such as "
     "Protocol Buffers [@protobuf] declare a message once for both sides, "
     "which removes the drift met in Outreach, but only for messages. "
     "Other tools declare once for a single area: Zephyr’s devicetree "
     "[@zephyr] and STM32CubeMX [@cubemx] for a microcontroller’s hardware, "
     "OpenAPI [@openapi] for an interface and Style Dictionary "
     "[@styledict] for design tokens. Each generates parts of a product "
     "from one declaration, but we know of none that checks a whole "
     "product, hardware included."),
    ("p",
     "Build systems such as Bazel [@bazel] and Nix [@nix] rebuild exactly "
     "what changed, identifying results by fingerprints of their inputs. "
     "They make sure outputs follow from files; they do not check that files "
     "agree with each other, so the same fact typed into two files builds "
     "without complaint."),
    ("p",
     "Coding agents follow context files such as AGENTS.md [@agentsmd], "
     "which describe a repository’s rules but do not enforce them, and "
     "their output is probabilistic and not guaranteed: a survey of 72 "
     "studies finds functional bugs, mostly in logic and meaning, the most "
     "common kind in AI-generated code [@gao]. A known answer is to pair a "
     "generator that can be wrong with a checker that cannot "
     "[@kambhampati], but we know of no existing checker that spans a whole "
     "product."),
    ("table", "Existing Approaches and Where They Stop",
     ("Approach", "Keeps in agreement", "Stops at"), [
        ("Model-based engineering", "views of one model",
         "graphical tool; code by hand"),
        ("Board–case links", "board and case", "two disciplines, two tools"),
        ("Design as code", "one discipline", "facts shared with others"),
        ("Interface languages", "messages, both sides", "everything else"),
        ("Build systems, CI", "outputs with files", "two files that disagree"),
        ("Fiducial", "every declared fact", "facts not yet declared"),
    ], (1500, 1550, 1500)),
    ("p",
     "Fiducial keeps what works in each (text that agents can write, "
     "outputs tied to inputs by fingerprints, established engines for hard "
     "problems, and checks that decide) and changes the unit. It tracks "
     "facts rather than files or tools, across every discipline of one "
     "product, so any declared fact (a message, a pin, a color, a size) has "
     "exactly one home."),

    ("h1", "The Approach"),
    ("p",
     "Fiducial treats a product as facts and artifacts. Facts are the "
     "decisions that define it: which pin a sensor uses, what a button says "
     "in each language, the size of the board, the layout of a message. "
     "Artifacts are everything built from them: code, schematics, boards, "
     "translations, enclosure models. Drift is when an artifact stops "
     "agreeing with a fact, or with another artifact that uses it "
     "(Fig. 1a). Three rules keep drift out (Fig. 1b)."),
    ("p",
     "Declare once. Each declared fact is written in exactly one place, a "
     "declaration: a typed text file in the repository, with a structure a "
     "program can check. A fact that exists once has no second copy to "
     "disagree with."),
    ("p",
     "Derive everything else. Every artifact that depends on a fact is "
     "generated from its declaration by one command and never edited by "
     "hand. Generation is meant to be deterministic, so any difference is "
     "meaningful: the placement search runs on one worker with a fixed seed "
     "and a pinned solver version. The autorouter is not, so KiCad’s "
     "design-rule check judges its output. TODO(Alex): confirm the "
     "freshness check does not hash the routed copper."),
    ("p",
     "Check every change. Before a change is accepted, checks confirm that "
     "every artifact still matches what its declarations produce and that "
     "the declarations are valid; a failure names the declaration and the "
     "file at fault."),
    ("p",
     "The rules check agreement, not truth. A fact declared wrongly (the "
     "wrong sensor address, a resistor of the wrong value) is copied "
     "faithfully into every artifact, and the product is then consistently "
     "wrong. Some such values break a rule and are refused, such as two "
     "colors with too little contrast, but whether a declaration is right "
     "is still decided by the person, the datasheet and the bench."),
    ("p",
     "These rules change the role of an AI agent: it still edits "
     "declarations and code, but it is no longer trusted to keep anything "
     "consistent. Many of its mistakes are caught before they reach the "
     "product, with a message precise enough to correct itself. Hence the "
     "name: a fiducial is a fixed reference point that everything is "
     "aligned to and compared against. The declarations are that reference "
     "for a product."),

    ("h1", "Implementation"),
    ("p",
     "Fiducial is a command-line tool, fid, built by one person working "
     "with AI agents. Fig. 2 shows how a change moves through "
     "its parts."),
    ("h2", "One Structure for Every Product"),
    ("p",
     "fid new creates a product repository with a fixed layout, and fid add "
     "enables capabilities. A capability packages one discipline: its "
     "declarations, the generator that turns them into artifacts, its checks "
     f"and instructions for agents. There are {_CAPS}, from web applications "
     "to firmware and hardware. Every product has the same structure, so "
     "a new one starts from parts that already work. Each also carries an "
     "AGENTS.md file and one instruction file per capability, and a hook "
     "stops an agent editing a generated file; a mistake that gets past the "
     "checks becomes a general rule for every product."),
    ("h2", "One Typed Language Underneath"),
    ("p",
     "The platform, and the code products share through it, is written in "
     "Rust, a compiled language with a strict type system: every mistake the "
     "compiler refuses is one the checks never have to catch, and its message "
     "names the file and line, the signal an agent needs. Rust needs no "
     "runtime or garbage collector, so one body of code (message formats, "
     "identifiers and typed physical quantities) compiles into a "
     "microcontroller’s firmware, into the fid tool, and to WebAssembly, "
     "which runs in the browser and on edge servers. The TypeScript types "
     "the web code uses are generated from the Rust ones."),
    ("h2", "Declarations and Generation"),
    ("p",
     "Declarations are plain text files in version control, from the "
     "strings of each language to every part of a board with the net each "
     "pin connects to. Coding agents read and write text well, and every "
     "change goes through ordinary code review. fid derive runs each "
     "capability’s generator, from a typed module of translations to a "
     "firmware pin map or a KiCad circuit board [@kicad]. A lock file "
     "records a cryptographic hash (a short fingerprint of a file’s "
     "contents) for every input and output, so the platform knows what each "
     "artifact was made from."),
    ("h2", "Checks"),
    ("p",
     "Checks run in CI, and a change is merged only when they pass. "
     "There are three kinds. Freshness: fid derive --check compares every "
     "recorded fingerprint with the repository and fails if a declaration "
     "changed without its artifacts being regenerated, or if a generated "
     "file was edited by hand. Rules: generation refuses declarations that "
     "break a rule, such as a language missing a string, two colors with too "
     "little contrast, parts that cost more than the declared limit, or a "
     "board that does not fit its case. Consumers: compilers and type "
     "checkers read the generated files, so code that still uses a renamed "
     "fact stops compiling."),
    ("h2", "Engines for Hard Problems"),
    ("p",
     "Some generation steps are real engineering problems: where to place "
     "the parts on a board, how to connect them, how to build a case around "
     "them. The platform writes the problem down and hands it to an "
     "established engine: a constraint-programming solver, CP-SAT [@cpsat], "
     "finds a placement that satisfies the rules or proves none exists; "
     "build123d [@build123d] builds the case; an autorouter routes the board "
     "and KiCad checks it. Every placement rule is one of eight general "
     "kinds, such as inside, apart or near, so a new product needs new "
     "declarations, not new code."),
    ("h1", "Example: A Sensor Stick"),
    ("p",
     "The repository contains a complete example: a USB-C stick that "
     "measures temperature and humidity and shows them on a web page "
     "(Fig. 3), with a printed case, a circuit board and Rust firmware. "
     "The person writes the declaration, the firmware’s logic and the page; "
     "the routed board, schematic, bill of materials, case, manufacturing "
     "files and the code both sides use to exchange readings are generated. Changing "
     "the scale of a reading is one changed line: the check fails until the "
     "product is regenerated, after which the firmware’s message code and the "
     "page’s have both followed with no other edit."),

    ("fig", "fig2-loop.png",
     "How a change moves through Fiducial: declarations are edited, "
     "artifacts are generated with established engines, and the checks "
     "accept the change or name the declaration at fault."),
    ("fig", "fig3-sensor-stick.png",
     "The sensor stick, generated from its declaration: (a) the closed case; "
     "(b) the routed board inside it, with the sensor under its vent."),
    ("h1", "Evaluation"),
    ("p",
     "To see whether the checks catch the mistakes that people and agents "
     "make, the test borrows mutation testing "
     "[@demillo]: a realistic mistake is introduced on purpose, the checks "
     "are run, and the result shows whether they noticed. Each mistake was "
     "applied to a clean copy of one of two products, a fresh web product "
     "in English and Serbian or the sensor stick, and the checks ran as in "
     "CI. Four sets of 20 were used. The first covers eight kinds of drift "
     "met while building Fiducial and was used to develop it, so it is not "
     "an independent test and is left out of the counts. All 60 mistakes of "
     "the other three sets were written by AI agents and committed to the "
     "repository before their first run, so none could be chosen after "
     "seeing the result: the second set on 28 October 2026 by the AI that "
     "built Fiducial, the third on 28 October "
     "and the fourth on 1 November by separate agents that read only the two "
     "products, not the platform, the fourth aimed at the classes the third "
     "revealed. As a baseline, every set also ran through the projects’ "
     "ordinary CI alone: builds, type checks and tests, without Fiducial’s "
     "derive and check steps. That CI compares no artifact against another, "
     "so its 0 of 60 shows the gap Fiducial fills, not a flaw in those tools. "
     "Correct changes tested that the checks do not fail without a reason. "
     f"Table II gives the results; Fiducial {RES_VERSION} is the release "
     "after the fixes."),
    ("table", "Injected Mistakes Caught",
     ("Mistakes", "CI alone", "First run", "After fixes, same sets"), "TABLE1",
     (1500, 800, 900, 1460)),
    ("p", "FINDINGS_A"),
    ("p", "FINDINGS_B"),
    ("p",
     "All four sets and their script are in the repository. Only first runs "
     "measure generalization, the fourth set was aimed at classes the third "
     "revealed, only two products have been built, and five correct-change "
     "runs are too few to show that the checks raise no false alarms. All 60 "
     "mistakes came from AI agents, possibly of the same model family as "
     "the AI that built Fiducial, so they may share its blind spots. The "
     "sensor stick is designed and checked but not built; TODO(Alex): "
     "confirm that no product has been manufactured and tested. The test "
     "measures the checks, not whether agents make fewer mistakes."),
    ("h1", "Conclusion"),
    ("p",
     "AI agents have made writing each part of a product fast, and that has "
     "moved the hard problem to keeping the parts in agreement. A "
     "probabilistic system cannot guarantee its output, and even a "
     "deterministic tool is only right about what it was told. Fiducial keeps the parts of a product consistent with each other, while "
     "whether each declared fact is true remains the person’s to judge. On "
     "60 mistakes it had not seen, its checks caught "
     f"{_POOL} ({_PCT}%) on first run where ordinary CI caught none. All "
     f"but one of the {_MISSES} misses fell into five classes, each now "
     f"closed by one mechanism; on the same sets, which produced those fixes, "
     f"{_AFTER} of the 60 are now caught, a measure of fit, and the last is "
     "a matter of meaning that is not yet declared. The next steps "
     "are to test it on mistakes written by other people, to give agents only the declarations a task needs (retrieval-augmented generation), to apply the platform to new products, "
     "including systems of several controllers and a server, and to measure "
     "and lower the tokens agents spend. "
     "Fiducial and its example are open source [@fiducial]."),
]

# Table II: each set without Fiducial, on its first run (first_runs.json),
# and on this release; the rest is read from the results.
def _ctl(res):
    c = [r for r in res["results"] if r["control"]]
    return f"{sum(r['pass'] for r in c)}/{len(c)}"


TABLE1 = [
    ("Set 1, development", f"{_B}/20", f"{_F1['original']}/20", f"{_N}/20"),
    ("Set 2, by the AI", f"{_HB}/20", f"{_F1['heldout']}/20", f"{_H}/20"),
    ("Set 3, by an agent", f"{_IB}/20", f"{_F1['independent']}/20", f"{_I}/20"),
    ("Set 4, by an agent", f"{_FB}/20", f"{_F1['fourth']}/20", f"{_F}/20"),
    ("Correct changes", _ctl(_BASE), "5/5", _ctl(_RESULTS)),
]

_MISSES = 60 - _POOL
FINDINGS_A = (
    f"On first run the checks caught {_POOL} of the 60 mistakes ({_PCT}%, "
    f"95% Wilson interval {_LO}–{_HI}%): {_F1['heldout']}, "
    f"{_F1['independent']} and {_F1['fourth']} of the three sets, which at "
    "20 each are indicative only. Ordinary CI caught none of them, because "
    "nothing in it compares one artifact with another. All but one of the "
    f"{_MISSES} misses fell into one of five classes: a declared value typed "
    "again in code, such as a footer spelling out the domain; the check "
    "itself removed, or run after a step that regenerates what it checks; a "
    "declaration typed so loosely that a misspelled key or a missing entry "
    "is accepted; a rule from outside the product, such as a flash chip’s "
    "size; and a generator reading a file nobody declared."
)
FINDINGS_B = (
    "Each class now has one mechanism: a scan for "
    "declared values in hand-written code; a check that CI still runs the "
    "gate, and runs it first; typed declarations that reject unknown keys "
    "and require what a rule needs; outside rules kept as data, from "
    "datasheets, fabrication limits and law; and generators confined to "
    f"their declared inputs. After these fixes, release {RES_VERSION} catches "
    f"{_AFTER} of the same 60 ({_H}, {_I} and {_F}), but these sets produced "
    "the fixes, so this measures fit, not generalization. The one it misses "
    "is a mistake of meaning: contrast rules allow a lower ratio for large "
    "text, and a color pair was relabeled as large text. It passes because "
    "which text is large is not yet declared."
)

REFS = {
    "claude": "Anthropic, “Claude Code documentation,” 2026. Available at: "
              "https://docs.anthropic.com/en/docs/claude-code",
    "codex": "OpenAI Codex. Available at: https://openai.com/codex/",
    "outreach": "A. Zdravković, Đ. Jovanović and P. Šubarević, “Outreach: Modular "
                "AI-enhanced off-grid mesh communication system for remote "
                "environments,” IEEESTEC – 18th Student Projects Conference, Niš, 2025.",
    "monorepo": "R. Potvin and J. Levenberg, “Why Google stores billions of "
                "lines of code in a single repository,” Commun. ACM, vol. 59, "
                "pp. 78–87, 2016.",
    "fiducial": f"A. Zdravković, Fiducial, release {VERSION}, 2026. Available at: "
                "https://github.com/AleksaZCodes/fiducial",
    "schmidt": "D. C. Schmidt, “Guest editor’s introduction: Model-driven "
               "engineering,” Computer, vol. 39, no. 2, pp. 25–31, 2006.",
    "brambilla": "M. Brambilla, J. Cabot and M. Wimmer, Model-Driven Software "
                 "Engineering in Practice, 2nd ed. Morgan & Claypool, 2017.",
    "sysml": "S. Friedenthal, A. Moore and R. Steiner, A Practical Guide to "
             "SysML, 3rd ed. Morgan Kaufmann, 2014.",
    "nuseibeh": "B. Nuseibeh, S. Easterbrook and A. Russo, “Leveraging "
                "inconsistency in software development,” Computer, vol. 33, "
                "no. 4, pp. 24–29, 2000.",
    "herzig": "S. J. I. Herzig, A. Qamar and C. J. J. Paredis, “An approach to "
              "identifying inconsistencies in model-based systems engineering,” "
              "Procedia Computer Science, vol. 28, pp. 354–362, 2014.",
    "altium": "Altium, MCAD CoDesigner, ECAD–MCAD collaboration. Available at: "
              "https://www.altium.com/",
    "atopile": "atopile, a language for designing circuit boards as code. "
               "Available at: https://github.com/atopile/atopile",
    "build123d": "build123d, a Python CAD library on the OpenCascade kernel. "
                 "Available at: https://github.com/gumyr/build123d",
    "protobuf": "Protocol Buffers. Available at: https://protobuf.dev/",
    "bazel": "Bazel, a build and test tool. Available at: https://bazel.build/",
    "nix": "E. Dolstra, M. de Jonge and E. Visser, “Nix: A safe and policy-free "
           "system for software deployment,” in Proc. LISA, 2004, pp. 79–92.",
    "agentsmd": "AGENTS.md, a format for guiding coding agents. Available at: "
                "https://agents.md/",
    "gao": "R. Gao, A. Tahir, P. Liang, T. Susnjak and F. Khomh, “A survey of "
           "bugs in AI-generated code,” arXiv:2512.05239, 2025.",
    "kambhampati": "S. Kambhampati et al., “Position: LLMs can’t plan, but can "
                   "help planning in LLM-Modulo frameworks,” in Proc. ICML, "
                   "PMLR 235, 2024.",
    "zephyr": "Zephyr Project, Devicetree: docs.zephyrproject.org/latest/build/dts",
    "cubemx": "STMicroelectronics, STM32CubeMX: st.com/en/development-tools/stm32cubemx",
    "openapi": "OpenAPI Initiative, OpenAPI Specification: spec.openapis.org",
    "styledict": "Style Dictionary: github.com/style-dictionary/style-dictionary",
    "kicad": "KiCad EDA. Available at: https://www.kicad.org/",
    "cpsat": "L. Perron and F. Didier, CP-SAT, Google OR-Tools. Available at: "
             "https://developers.google.com/optimization/cp/cp_solver",
    "demillo": "R. A. DeMillo, R. J. Lipton and F. G. Sayward, “Hints on test data "
               "selection: Help for the practicing programmer,” Computer, vol. 11, "
               "no. 4, pp. 34–41, 1978.",
}

# Number references by first citation, as IEEE style requires: [@key] in the
# text becomes [n], and the list is printed in that order.
_ORDER = []


def _cite(text):
    def num(m):
        if m.group(1) not in _ORDER:
            _ORDER.append(m.group(1))
        return f"[{_ORDER.index(m.group(1)) + 1}]"
    return _re.sub(r"\[@(\w+)\]", num, text)


BODY = [tuple(_cite(x) if isinstance(x, str) and i else x for i, x in enumerate(item))
        for item in BODY]
FINDINGS_A, FINDINGS_B = _cite(FINDINGS_A), _cite(FINDINGS_B)
_unused = set(REFS) - set(_ORDER)
assert not _unused, f"references never cited: {_unused}"
REFERENCES = [REFS[k] for k in _ORDER]

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
# The template centres the title in a box from 900 twips to 836 twips short
# of the right edge: a 1872-twip right margin and a -1036 indent. The body's
# margins are set to 907 above, so the indent that keeps the same box is
# -71; left at -1036 it pushes the title 0.4 in right of centre.
title._p.pPr.find(qn("w:ind")).set(qn("w:end"), str(-(1036 - (1872 - 907))))

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
