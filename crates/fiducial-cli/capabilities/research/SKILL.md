# fiducial:research — Research & Authoring

This product has the `research` capability. **One declaration, one submittable
artifact** — title, authors, venue, and a references directory declared once in
`[paper]`; bibliography and the typeset document derived from them by the
`fid-research` pipeline.

## The loop

```
references/<key>.bib   ← you write or import these
[paper] in fiducial.toml
  ↓  fid derive        (pipeline: research — executor fid-research)
paper/bibliography.bib  ← merged, de-duplicated bibliography
paper/main.pdf          ← typeset via Pandoc (IEEE/ACM/APA template per venue)
```

## The declaration

```toml
[paper]
title           = "Fiducial: A Declaration-First Platform for Cross-Domain Products"
authors         = ["Aleksa Živković"]
venue           = "IEEE"           # controls the typeset template: IEEE, ACM, APA, plain
references_dir  = "references/"    # .bib files here; also accepts .cff, .json (CSL-JSON)
manuscript      = "paper/main.md"  # Markdown source; if absent, fid-research scaffolds one
output_dir      = "paper/"
```

## Reference formats accepted

| Format | Extension | Notes |
|--------|-----------|-------|
| BibTeX | `.bib` | Most common; all fields passed through |
| CSL-JSON | `.json` | Citation Style Language; converted to BibTeX on derive |
| CFF | `.cff` | `CITATION.cff`-style; for citing other Fiducial repositories |
| DOI | `.doi.txt` | One DOI per line; resolved to BibTeX via `https://doi.org/` Content-Type negotiation |

## Manuscript format

The manuscript is Markdown with Pandoc-style citations:

```markdown
# Introduction

Cross-domain product development presents unique challenges [@fiducial2026].
The approach of declaring facts once and deriving all artifacts from them
has been applied successfully in electronic design [@ritchie1978c].
```

`fid-research` calls Pandoc with the resolved bibliography and the venue
template. The author never touches LaTeX boilerplate — that is what the
pipeline is for.

## DOI resolution

`.doi.txt` files are resolved during `fid derive`. Each DOI is fetched
from `https://doi.org/<doi>` with `Accept: application/x-bibtex`, and the
result is cached in `.fiducial/doi-cache/` so subsequent derives are
offline-capable.

If a DOI resolves to a 404 or the network is unavailable and no cache entry
exists, `fid derive` fails with the DOI listed, not silently missing. A
stale bibliography is the same class of bug as a stale generated type.

## Fiducial is its own first customer

The intended first paper is an IEEE student conference submission about the
platform itself. `docs/specs/2026-09-14-disclosure-authorship-and-citation.md`
frames the contribution, the evidence, and what not to claim — that document
is the brief; this capability is the tooling that lets the paper be written
in the repository rather than beside it.

## Sequencing

The **Zenodo DOI** (see ROADMAP Out-of-band section) should exist before the
paper is submitted — the paper cites the repository, and a repository without
a DOI cannot be cited correctly. `CITATION.cff` carries no `version` or
`date-released` until the first tagged release; derive both at release time
rather than hand-writing them.

## Dependencies

- Pandoc must be available on `$PATH` for PDF output. HTML output works
  without it.
- DOI resolution requires outbound HTTPS to `doi.org`.
