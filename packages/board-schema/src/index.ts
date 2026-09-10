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

/** Which board edge — and so which case wall — a feature sits on. */
export type Side = 'north' | 'south' | 'east' | 'west'

/**
 * Body envelopes for the connector families `type` may name, in millimetres.
 *
 * Mirrors `fiducial_geometry::CONNECTOR_OPENINGS`. These are body dimensions,
 * not opening dimensions — the process clearance is added when the hole is cut.
 */
export const CONNECTOR_OPENINGS: Record<string, { width_mm: number; height_mm: number }> = {
  'usb-c': { width_mm: 8.94, height_mm: 3.26 },
  'micro-usb': { width_mm: 7.5, height_mm: 2.9 },
  'usb-a': { width_mm: 13.2, height_mm: 5.8 },
  swd: { width_mm: 10.16, height_mm: 8.5 },
  qwiic: { width_mm: 6.25, height_mm: 4.25 },
  'jst-ph': { width_mm: 7.8, height_mm: 6.0 },
  microsd: { width_mm: 12.0, height_mm: 1.6 },
  rj45: { width_mm: 15.9, height_mm: 13.5 },
  'barrel-jack': { width_mm: 9.0, height_mm: 11.0 },
}

/**
 * Where a connector meets the case wall.
 *
 * `side` and `offset_mm` are in board coordinates: the board sits in the first
 * quadrant and `offset_mm` is measured along the named edge from the board's
 * origin corner. Positioning by the board means the declaration never has to
 * know how thick the seal made the wall.
 */
export interface Mount {
  /** Board edge the connector faces. */
  side: Side
  /** Centre of the connector along that edge, from the board's origin corner. */
  offset_mm: number
  /** Body width across the edge. Defaults to the family envelope. */
  width_mm?: number
  /** Body height. Defaults to the family envelope. */
  height_mm?: number
  /** Opening floor above the board's top surface; negative for mid-mount. */
  z_offset_mm?: number
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
  /**
   * Where this connector meets the case wall, when it needs an opening.
   *
   * Optional: a header meant to be reached with the lid off has no mount and
   * gets no hole. Omitting it is the safe default — an unnecessary opening is
   * a leak.
   */
  mount?: Mount
}

/**
 * Body envelope for a mount: the declaration if given, otherwise the family
 * default for `kind`.
 *
 * Returns `undefined` when neither is available — the caller must then reject
 * the declaration rather than guess a size for an unknown family.
 */
export function mountEnvelope(
  mount: Mount,
  kind: string
): { width_mm: number; height_mm: number } | undefined {
  const fallback = CONNECTOR_OPENINGS[kind]
  const width_mm = mount.width_mm ?? fallback?.width_mm
  const height_mm = mount.height_mm ?? fallback?.height_mm
  if (width_mm === undefined || height_mm === undefined) return undefined
  return { width_mm, height_mm }
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
  /**
   * Height of the posts the board rests on, in millimetres.
   *
   * Omit and the board sits on the cavity floor. Declaring a height raises the
   * board onto four corner posts and grows the case by the same amount,
   * because headroom is measured above the board.
   */
  standoff_height_mm?: number
  /** Footprint of each standoff post, square, in millimetres. */
  standoff_size_mm?: number
  /**
   * Screw shaft diameter for the four corner fasteners that retain the lid.
   *
   * Omit and nothing clamps the lid down: the gasket only compresses while
   * something external holds it shut. Declaring a diameter widens the outer lip
   * enough to carry a hole with a printable wall either side, which thickens
   * the case wall — so it is opt-in rather than assumed.
   */
  fastener_diameter_mm?: number
}

/**
 * Physical board outline — the declaration the mesh pipeline derives from.
 *
 * When present, `fid derive` generates the sealed case parts from it
 * (`case-base.stl`, `case-lid.stl`, `gasket.stl`, `case.glb`), sized by
 * `tolerance`, with an opening punched for every connector that declares a
 * `mount`.
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
        'standoff_height_mm',
        'standoff_size_mm',
        'fastener_diameter_mm',
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

  const SIDES: Side[] = ['north', 'south', 'east', 'west']
  for (const c of bi.connectors) {
    const m = c.mount
    if (!m) continue
    // An outline is what a mount is measured against; a mount without one has
    // nothing to cut and would be silently dropped.
    if (!bi.outline) {
      throw new Error(`connector ${c.id} declares a mount but the board declares no outline`)
    }
    if (!SIDES.includes(m.side)) {
      throw new Error(
        `connector ${c.id}: unknown side ${JSON.stringify(m.side)} (expected north, south, east or west)`
      )
    }
    if (!(m.offset_mm >= 0)) {
      throw new Error(`connector ${c.id}: offset_mm must be >= 0, got ${m.offset_mm}`)
    }
    for (const field of ['width_mm', 'height_mm'] as const) {
      const v = m[field]
      if (v !== undefined && !(v > 0)) {
        throw new Error(`connector ${c.id}: mount.${field} must be > 0, got ${v}`)
      }
    }
    if (m.z_offset_mm !== undefined && Number.isNaN(m.z_offset_mm)) {
      throw new Error(`connector ${c.id}: mount.z_offset_mm is not a number`)
    }
    // A family with no entry in the opening table cannot imply a size, so the
    // declaration has to supply one rather than have one guessed.
    if (!mountEnvelope(m, c.type)) {
      throw new Error(
        `connector ${c.id}: no body envelope known for type ${JSON.stringify(c.type)} — ` +
          `declare mount.width_mm and mount.height_mm`
      )
    }
  }

  return bi
}
