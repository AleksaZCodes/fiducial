---
"@fiducial/advisor": minor
---

Argue with a product's thesis: `fid advise --thesis`.

Seven advisory questions about the one claim a product is built to test — is it
falsifiable, would anyone disagree, does it describe instead of claiming, does
the stated falsifier actually falsify it, is the named dissent a strawman, does
the product's own copy sell something else, and does that copy claim more than
the evidence supports.

The declaration is read through `fid thesis --json` rather than re-parsed here,
so there is one parser for it. A question whose input is missing is not asked:
no `disagrees` means no strawman question, and no hand-written copy means
neither copy question. A product with no thesis sends nothing and exits 0.

`weigh()` now takes a `weights` option so a question set brings its own
ranking; it defaults to `DIFF_WEIGHTS`, so existing callers are unchanged.
