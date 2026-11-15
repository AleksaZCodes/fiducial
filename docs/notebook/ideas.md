# Ideas and sparks

*Mirrored from the maintainer's planning tool, synced 2026-11-15. Stage and
next action are tracked there; the substance is here.*

## Fiducial as one shared-context platform (Studio) · Spark


One place for any project, product-first: app, research, papers, learning,
content, with shared context across chat and code. Own harness on the Claude
Agent SDK and an ACP-style client, a cross-platform UI, an artifact layer on
open formats (SKILL.md, DTCG tokens, MCP Apps `ui://`, plain md/JSON).
GitHub and Cloudflare connected; the user owns everything.

**Nov 13.** First real use case: idea → production-ready website or simple
web app, shipped and sold, no AI slop, some upfront thinking built in. Needs
domains, tunnelling, a working browser. Later firmware, PCB, CAD, mobile; a
handoff/sale platform; opinionated so the user need not learn to use AI well;
transparent at every step.

**Nov 15 (spoken brief).** A meta-harness, not an IDE:

- **See everything in one web app**, lightweight, any computer: the running
  site or app beside its files; the CMS; the configuration; the Worker behind
  it; how things connect; the PRs; the agents' chats and what they did.
- **Connected to Cloudflare and GitHub** first (domains via Cloudflare DNS).
  Notion maybe later; calendar out of scope.
- **Open a file and see its page**, with the dev server behind it; edit code
  or edit the page (WYSIWYG for content); configuration, capabilities and
  modules stay in the Fiducial platform, not in the CMS, separated by concern.
- **Claude Code inside**, with its own auth; optionally a development server
  with real compute, ideally without egress, so work is fast and token-cheap.
- **Zed-like editor** if one is needed. Finding: Zed has no web build or
  embeddable component (native Rust); the practical web editors are
  CodeMirror 6 or Monaco, styled to taste.
- **Knowledge in the same place**: AI chats, brainstorming, tasks, design
  system principles, the product's context, so the workflow as a whole does
  not drift, not only the code.
- **Structured and finite** rather than infinitely flexible: it only advances
  the product.
- Uses: a personal site fast and cheap; sites for clients (the market there:
  white-label AI site platforms such as 10Web, Duda, Lindo.ai, Brizy, all
  hosted builders that own the data, the opposite of this); physical products;
  a slide deck for any of them.

**Order (Nov 15).** Studio is the home every code project lives in: tech
products, research, anything built with code, including the maintainer's own
tools and devices. It comes first because it is what makes the rest visible,
navigable and lower in cognitive load while building it.

Next: decide whether a thin, read-only viewer is worth a cheap first test.

## Prototype first, living models at the core · Spark

Spoken Nov 15. People and agents disagree most on how a thing should look and
work, and prose specs never get reread. So:

- **A working prototype before the build**: a dummy UI with the real design
  system and components (fully styled shadcn), clickable, with real-looking
  data, images and graphs, agreed on first, then reused as the product's code
  and structure rather than thrown away.
- **Living models at the core**: structure, flows and screens declared as data
  in a strict, standardized notation, derived into diagrams and wireframes, and
  checked against the app, so they are current and used even in the simplest
  places. Specs and decisions stay as history.
- **Standards to stand on**: model-based systems engineering (MBSE), with SysML
  v2 (textual notation) for whole systems and the C4 model for software
  architecture; diagrams-as-code renderers rather than drawn pictures.
- Fits the core rule: a model is a declaration, a diagram is an artifact, and a
  route or screen the model lists but the app lacks fails the check.
- **Models means real things to look at and edit**: graphs, actual UI,
  canvases, with a good editor around them; extensible, like a design tool
  built into the artifacts rather than a separate app.
- **Open-source pieces to build on** (researched Nov 15, not yet tried), each
  usable by a person and by an agent through MCP (Claude.ai included):
  - *Penpot*: open-source design tool, open SVG-based format, an official MCP
    server in its main repository. The closest to a design canvas both sides
    can edit.
  - *Excalidraw* and *tldraw*: canvases for flows and sketches; MCP servers
    exist, and tldraw is an SDK for embedding a canvas in our own UI.
  - *Onlook*: open-source visual editor that edits a real React, Tailwind and
    shadcn codebase, so the prototype is the product's code, not a picture.
  - *Puck*: an embeddable visual page editor for React that stores pages as
    JSON, which is a declaration in this platform's sense.
  - Diagrams as code: *Structurizr DSL* for C4, *Mermaid*, and the SysML v2
    textual notation for whole systems.

## Fiducial's edge: System One advisory review for correctness · Spark


The paper proves consistency, not truth. Typed, calibrated answers to
structured questions (`fid advise`) could catch wrong-but-consistent facts.
AI advises, never derives or gates. Unevaluated: evaluate with injected
mistakes that stay consistent across artifacts, against a plain-LLM baseline,
after the conference.

## Fiducial conference demo: 10-second change-and-break visual · Spark


Cheapest first: a real 30 s recording plus the sensor stick and printed case;
then a one-file "without / with Fiducial" animation; a tap-through app only
after the conference.

## PCB tooling · Spark


Modular PCB-making machine, self-assembling circuits and breadboard,
footprints and 3D models generated from images.

## Fiducial weather board · Spark


A board for meteorological forecasts and weather balloons.
