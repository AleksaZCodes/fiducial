# One GitHub release per platform version

**Date:** 2026-10-02
**Status:** accepted

## Context

Fiducial ships as fifteen crates on crates.io (one workspace version) and
thirteen npm packages (each versioned by changesets). On GitHub:

- every npm publish pushed a per-package tag (`@fiducial/tokens@0.3.0`);
- changesets also created a GitHub *release* per package — but only six
  exist (adapters, advisor, tokens, realtime), because earlier release runs
  failed between tagging and releasing;
- the crates got neither a tag nor a release.

So "which Fiducial is this?" had no answer on GitHub, and connecting Zenodo
— which mints a DOI for every GitHub release — would have minted one per npm
package bump instead of one per platform version.

## Decision

- **One release per platform version**, tagged `vX.Y.Z` from the workspace
  version in `Cargo.toml`, created by `scripts/github-release.sh` as the
  last step of `release.yml`, only after `verify-published.sh` confirmed the
  registries hold it. Its notes list every crate and every public npm package
  at the version it carries, read from the repository.
- **Per-package releases off** (`create-github-releases: false`). Per-package
  tags stay: changesets needs them to know what it published.
- The six existing per-package releases are left as history.

## Citation

With Zenodo connected to the repository, each `vX.Y.Z` release gets a
*version DOI* (that exact code, forever) and the project a *concept DOI*
(always the latest). A paper cites the version DOI of the release it
describes. `CITATION.cff` and `.zenodo.json` already carry the metadata;
Zenodo adds version and date from the release.
