# Changesets

This directory stores changeset files — one per PR that warrants a version bump.

## Workflow

1. `pnpm changeset` — describe what changed and which packages it affects.
2. Commit the generated `.md` file with the PR.
3. After merging, the release CI creates a "Release PR" that bumps versions and
   collects changelogs.
4. Merge the Release PR → CI publishes to npm and crates.io.

## Rules (from Appendix A.2)

A changeset is only useful if the release step actually runs. Before adding any
rule or gate, ask: **who reads the output, and when?**

The answer here: the Release PR is the reader, and it is created automatically by
`changeset-bot` after every merge to `main`.
