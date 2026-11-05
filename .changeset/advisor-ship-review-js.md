---
"@fiducial/advisor": patch
---

Fix `@fiducial/advisor@0.4.0`, which did not run when installed from npm. `src/review.js`, the reasoning-model review module that `cli.js` imports, was missing from the package's `files` allowlist, so every command, `status` included, crashed at import on a clean install. It is now published, and a test packs the package and checks that every module the CLI reaches, and every non-test module in `src`, is in the tarball. Use 0.4.1 or later; 0.4.0 should be treated as broken.
