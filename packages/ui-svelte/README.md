# @fiducial/ui-svelte

**Component registry source — Svelte. Copied in, not installed.**

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## How this package is meant to be used

Not as a runtime dependency. `fid add component <name> --framework svelte` copies
the source into your product, where you own and edit it — the same copy-in model
as [`@fiducial/ui-react`](../ui-react), for the same reason: a component you
cannot modify is a trap the first time a product needs a variant its author did
not foresee.

```sh
fid add component button --framework svelte
```

## Components

`Button`, `Card`, `Badge`, `Dialog` — Svelte 5, using runes (`$props`, `$effect`).

`Dialog` wraps the native `<dialog>` element and opens it with `showModal()`,
which supplies the focus trap, the Escape binding and the backdrop from the
platform rather than from a library.

## Styling

Every component reads the CSS variables published by
[`@fiducial/tokens`](../tokens). The React registry reads the *same* variables,
so both frameworks render one design system from one declaration — which is the
whole reason the tokens package exists separately from either.

## A note on the TypeScript version

This package pins `typescript@~6` with `@typescript/native` aliased to 7, while
the rest of the workspace takes TypeScript 7 from the pnpm catalog. That is not
drift: `svelte-check` requires both versions present plus the `--tsgo` flag to
run under TypeScript 7, and it refuses to start otherwise.

The alternative was leaving this package unchecked, which is what happened before
— its `typecheck` script was an `echo` that reported success, and its `tsconfig`
excluded the only file it would have checked. Turning the check on found a real
accessibility defect in `Dialog.svelte` on the first run.

## License

MIT
