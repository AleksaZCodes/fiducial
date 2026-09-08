# @fiducial/viewer3d-react

A React component that renders a Fiducial board or enclosure GLB in a Three.js
WebGL canvas, with orbit controls and automatic camera framing.

## Install

```sh
pnpm add @fiducial/viewer3d-react three react
```

`three` and `react` are peer dependencies — this package does not bundle either,
so it uses whichever copy your app already has.

## Usage

```tsx
import { BoardViewer } from '@fiducial/viewer3d-react'

export default function Page() {
  return <BoardViewer src="/board.glb" width={800} height={500} />
}
```

In Next.js the App Router renders on the server by default; this component needs
a DOM, so mark the file `'use client'`.

## Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `src` | `string` | — | URL of the `.glb` to load (required) |
| `width` | `number` | `600` | Canvas width in pixels |
| `height` | `number` | `400` | Canvas height in pixels |
| `background` | `string` | `'#1a1a2e'` | Scene background colour |
| `className` | `string` | — | Class applied to the container |
| `style` | `CSSProperties` | — | Inline style applied to the container |

The camera frames the model from its bounding box after load, so a 10 mm part and
a 400 mm part both fill the viewport without per-model tuning. The renderer,
controls, and animation loop are disposed on unmount.

## Where the GLB comes from

`fid derive` generates `enclosure/board.glb` from the `outline` block declared in
`board/board.interface.json`. It is a derived artifact — regenerate it rather
than editing it, and copy it where your app serves static files:

```sh
cp enclosure/board.glb apps/web/public/board.glb
```

The output is standard glTF 2.0 binary, so it also opens in Blender and
`<model-viewer>` unchanged.

## License

MIT
