# @fiducial/advisor

Advisory review for [Fiducial](https://github.com/AleksaZCodes/fiducial) products.
Reached as `fid advise`.

## What it is for

It asks a decision model the questions an exact check cannot answer:

- a fact declared twice under two different names
- logic sitting above the layer it could reach
- a physical quantity declared without a unit or tolerance
- an abstraction with no escape hatch
- a comment explaining *what* instead of *why*
- a change that does more than was asked, or is left half-finished

## What it deliberately does not ask

Anything already decided exactly. A hand-edited artifact is `fid derive --check`.
A missing translation is the i18n pipeline. Contrast ratios are arithmetic. A
push to main is the guard. Spending a probabilistic answer where a certain one
exists is a regression dressed as a feature, and a test in this package enforces
that boundary.

Every check is a hybrid: code narrows candidates deterministically and the model
judges only what survives. A question whose input is missing is not asked at all.

## It cannot break anything

It writes no file, touches no artifact, and exits 0 even with findings —
including when the key is missing, the network is down, or the model is
overloaded. `fid derive` is byte-identical with and without a key, and
`crates/fiducial-cli/tests/determinism.rs` proves it by deriving a product twice
and hashing both trees. A gate whose verdict varies is not a gate, so **AI in
this platform advises and never derives.**

`--strict` opts into a non-zero exit for a hook of your own. Never wire it into
a gate that must not flake.

## Usage

```sh
fid advise key set          # store an OpenRouter or TypeSafe key
fid advise                  # review uncommitted changes
fid advise --task "..."     # judge scope against what was asked
fid advise --base main      # review the whole branch
fid advise --facts          # duplicate-check new declarations
fid advise --dry-run        # print the exact request body, send nothing
```

`--dry-run` exists because this sends your diff to a third-party API, and
"trust me" is not an acceptable answer to what leaves the machine.

## License

MIT
