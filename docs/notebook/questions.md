# Open questions

*Mirrored from the maintainer's planning tool, synced 2026-11-15.*

## What exactly goes wrong with Fiducial when building websites, and what is the cheapest reliable workflow for landing pages and admin panels? · Open


**Thinking (Nov 9).** Unknown whether it is architecture, rules, model choice,
tooling or workflow. Deliver one real website with the current setup and
measure time, AI use and manual fixes.

**Evidence since (Nov 13–15).** Building fresh products end to end found that
much of it was the platform: the sitemap listed pages that did not exist, the
legal module was written outside the app, fonts never loaded, the SvelteKit
templates had never compiled, `fid add deploy` targeted a Worker that did not
exist, and nothing wired content into pages. 0.15.0 fixed those and added
`fid new <name> --shape site`. Still unmeasured: time and manual fixes on a
real client site.
