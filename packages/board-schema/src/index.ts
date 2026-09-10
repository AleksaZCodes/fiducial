/**
 * TypeScript types for board.interface.json.
 *
 * Mirrors the Rust `fiducial_eda::BoardInterface` type — changes to the Rust
 * schema must be reflected here. The schema version is "1.0".
 */

/** Direction of a pin's signal, matching the Rust `PinDirection` enum. */
export type PinDirection =
  | 'input'
  | 'output'
  | 'bidirectional'
  | 'power_in'
  | 'power_out'

/** A single pin on a connector. */
export interface Pin {
  /** Physical pin number. */
  number: number
  /** Signal name (e.g. "VBUS", "D+", "SWDIO"). */
  name: string
  /** Net this pin belongs to. */
  net: string
  /** Signal direction. */
  direction: PinDirection
}

/** A connector or port on the board. */
export interface Connector {
  /** Reference designator (e.g. "J1"). */
  id: string
  /** Human-readable connector name. */
  name: string
  /** Connector family type (e.g. "usb-c", "swd", "qwiic"). */
  type: string
  /** All pins on this connector. */
  pins: Pin[]
}

/** A named group of related signal nets. */
export interface NetClass {
  /** Net class name (e.g. "power", "usb", "debug"). */
  name: string
  /** All net names in this class. */
  nets: string[]
}

/** Manufacturing process driving enclosure tolerances. */
export type ToleranceClass = 'fdm' | 'resin' | 'cnc'

/**
 * Overrides for the generated case.
 *
 * These are the values a manufacturing process cannot imply: how tall the
 * tallest component is, and what gasket stock the product uses. Everything
 * else is derived from the tolerance class.
 */
export interface EnclosureOptions {
  /** Vertical space above the board, in millimetres. Raise for tall parts. */
  headroom_mm?: number
  /** Lid plate thickness in millimetres. */
  lid_thickness_mm?: number
  /** Gasket cross-section width in millimetres. */
  gasket_width_mm?: number
  /** Gasket cross-section height in millimetres, uncompressed. */
  gasket_height_mm?: number
  /** Fraction of gasket height squeezed when closed. Must be in (0, 1). */
  gasket_compression?: number
}

/**
 * Physical board outline — the declaration the mesh pipeline derives from.
 *
 * When present, `fid derive` generates the sealed case parts from it
 * (`case-base.stl`, `case-lid.stl`, `gasket.stl`, `case.glb`), sized by
 * `tolerance`.
 */
export interface Outline {
  /** Board width in millimetres. */
  width_mm: number
  /** Board height in millimetres. */
  height_mm: number
  /** PCB thickness in millimetres (defaults to 1.6 when omitted). */
  thickness_mm?: number
  /** Enclosure manufacturing process (defaults to "fdm" when omitted). */
  tolerance?: ToleranceClass
  /** Case overrides the process cannot imply. */
  enclosure?: EnclosureOptions
}

/** Board identity block. */
export interface Board {
  /** Machine-readable board identifier. */
  name: string
  /** PCB revision (e.g. "A", "1.2"). Optional. */
  revision?: string
  /** Human-readable board description. Optional. */
  description?: string
}

/**
 * Root type for `board/board.interface.json`.
 *
 * Declares every connector, pin, and net class on the board — the
 * machine-readable interface between the EDA pipeline and the rest of
 * the platform.
 */
export interface BoardInterface {
  /** Schema version — currently "1.0". */
  schema_version: string
  /** Board identity. */
  board: Board
  /** Physical outline, when the board drives enclosure generation. */
  outline?: Outline
  /** All external connectors and their pins. */
  connectors: Connector[]
  /** Net class groupings. */
  net_classes: NetClass[]
}

/**
 * Parse and validate a `board.interface.json` string.
 *
 * Throws if the JSON is malformed or schema_version is not "1.0".
 */
export function parseBoardInterface(json: string): BoardInterface {
  const bi = JSON.parse(json) as BoardInterface
  if (bi.schema_version !== '1.0') {
    throw new Error(`unsupported schema_version: ${JSON.stringify(bi.schema_version)}`)
  }
  if (!bi.board?.name) throw new Error('board.name is required')
  if (!Array.isArray(bi.connectors)) throw new Error('connectors must be an array')
  if (!Array.isArray(bi.net_classes)) throw new Error('net_classes must be an array')
  if (bi.outline) {
    const { width_mm, height_mm, thickness_mm, tolerance } = bi.outline
    // Mirrors fiducial_eda::validate — a non-positive dimension yields a
    // degenerate mesh that slicers accept and then print as nothing.
    if (!(width_mm > 0) || !(height_mm > 0) || (thickness_mm !== undefined && !(thickness_mm > 0))) {
      throw new Error('outline width_mm, height_mm and thickness_mm must all be > 0')
    }
    if (tolerance !== undefined && !['fdm', 'resin', 'cnc'].includes(tolerance)) {
      throw new Error(`unknown tolerance ${JSON.stringify(tolerance)} (expected fdm, resin or cnc)`)
    }

    const e = bi.outline.enclosure
    if (e) {
      for (const field of [
        'headroom_mm',
        'lid_thickness_mm',
        'gasket_width_mm',
        'gasket_height_mm',
      ] as const) {
        const v = e[field]
        if (v !== undefined && !(v > 0)) {
          throw new Error(`enclosure.${field} must be > 0, got ${v}`)
        }
      }
      // 0 never squeezes the gasket and 1 crushes it flat; neither seals.
      const c = e.gasket_compression
      if (c !== undefined && !(c > 0 && c < 1)) {
        throw new Error(`enclosure.gasket_compression must be between 0 and 1 (exclusive), got ${c}`)
      }
    }
  }
  return bi
}
