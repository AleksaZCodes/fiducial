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
  return bi
}
