// Derived by fid-topology from topology.toml — do not edit.

export type PointKind = "sensor" | "actuator" | "contact"

export interface PointInfo {
  readonly id: number
  readonly name: string
  readonly kind: PointKind
  readonly quantity: string | null
  readonly unit: string | null
  readonly range: readonly [number, number] | null
  readonly decides: readonly string[]
}

export interface ParameterInfo {
  readonly id: number
  readonly name: string
  readonly unit: string
  readonly min: number
  readonly max: number
  readonly default: number
  readonly rule: string
  readonly limits: string | null
}

export const POINTS = {
  "classroom": [
    { id: 1, name: "co2", kind: "sensor", quantity: "co2", unit: "ppm", range: [400, 5000], decides: ["recuperator"] },
    { id: 2, name: "inside_temp", kind: "sensor", quantity: "temperature", unit: "°C", range: [-10, 60], decides: [] },
    { id: 3, name: "humidity", kind: "sensor", quantity: "relative_humidity", unit: "%", range: [0, 100], decides: [] },
    { id: 4, name: "outside_light", kind: "sensor", quantity: "illuminance", unit: "lx", range: [0, 65535], decides: ["blinds"] },
    { id: 5, name: "outside_temp", kind: "sensor", quantity: "temperature", unit: "°C", range: [-30, 60], decides: ["blinds"] },
    { id: 6, name: "classroom_all_stop", kind: "contact", quantity: null, unit: null, range: null, decides: ["blinds", "recuperator", "alarm"] },
    { id: 7, name: "blind_auto", kind: "contact", quantity: null, unit: null, range: null, decides: ["blinds"] },
    { id: 8, name: "recuperator_auto", kind: "contact", quantity: null, unit: null, range: null, decides: ["recuperator"] },
    { id: 9, name: "blind_up", kind: "actuator", quantity: null, unit: null, range: null, decides: [] },
    { id: 10, name: "blind_down", kind: "actuator", quantity: null, unit: null, range: null, decides: [] },
    { id: 11, name: "recuperator", kind: "actuator", quantity: null, unit: null, range: null, decides: [] },
    { id: 12, name: "buzzer", kind: "actuator", quantity: null, unit: null, range: null, decides: [] },
  ],
  "roof": [
    { id: 1, name: "soil_1", kind: "sensor", quantity: "soil_moisture", unit: "%", range: [0, 100], decides: ["irrigation"] },
    { id: 2, name: "soil_2", kind: "sensor", quantity: "soil_moisture", unit: "%", range: [0, 100], decides: ["irrigation"] },
    { id: 3, name: "roof_bare", kind: "sensor", quantity: "temperature", unit: "°C", range: [-30, 90], decides: [] },
    { id: 4, name: "roof_green", kind: "sensor", quantity: "temperature", unit: "°C", range: [-30, 90], decides: [] },
    { id: 5, name: "float_low", kind: "contact", quantity: null, unit: null, range: null, decides: ["irrigation"] },
    { id: 6, name: "float_high", kind: "contact", quantity: null, unit: null, range: null, decides: ["irrigation"] },
    { id: 7, name: "roof_all_stop", kind: "contact", quantity: null, unit: null, range: null, decides: ["irrigation"] },
    { id: 8, name: "pump_auto", kind: "contact", quantity: null, unit: null, range: null, decides: ["irrigation"] },
    { id: 9, name: "valve_auto", kind: "contact", quantity: null, unit: null, range: null, decides: ["irrigation"] },
    { id: 10, name: "pump", kind: "actuator", quantity: null, unit: null, range: null, decides: [] },
    { id: 11, name: "valve", kind: "actuator", quantity: null, unit: null, range: null, decides: [] },
  ],
} as const satisfies Record<string, readonly PointInfo[]>

export const PARAMETERS = {
  "classroom": [
    { id: 1, name: "co2_on", unit: "ppm", min: 600, max: 2000, default: 1000, rule: "recuperator", limits: "co2" },
    { id: 2, name: "co2_off", unit: "ppm", min: 400, max: 1800, default: 800, rule: "recuperator", limits: "co2" },
    { id: 3, name: "blind_warm", unit: "°C", min: 15, max: 35, default: 24, rule: "blinds", limits: "outside_temp" },
    { id: 4, name: "blind_bright", unit: "lx", min: 1000, max: 60000, default: 20000, rule: "blinds", limits: "outside_light" },
    { id: 5, name: "blind_partial", unit: "s", min: 1, max: 60, default: 8, rule: "blinds", limits: null },
  ],
  "roof": [
    { id: 1, name: "soil_start", unit: "%", min: 10, max: 60, default: 30, rule: "irrigation", limits: "soil_1" },
    { id: 2, name: "soil_stop", unit: "%", min: 20, max: 80, default: 40, rule: "irrigation", limits: "soil_1" },
  ],
} as const satisfies Record<string, readonly ParameterInfo[]>

export const NODES = [
  { id: "roof", system: "skola", firmware: "roof" },
  { id: "classroom_1", system: "skola", firmware: "classroom" },
  { id: "server", system: "skola", firmware: null },
] as const

export const RULES = [
  { id: "irrigation", scope: "roof", reads: ["soil_1", "soil_2", "float_low", "float_high", "roof_all_stop", "pump_auto", "valve_auto"], drives: ["pump", "valve"], doc: "The pump runs from soil_start until soil_stop on the average of both probes, never dry; an empty tank opens the valve until the high float." },
  { id: "blinds", scope: "classroom", reads: ["outside_temp", "outside_light", "classroom_all_stop", "blind_auto"], drives: ["blind_up", "blind_down"], doc: "Warm is above blind_warm, bright above blind_bright; with the room occupied (timetable) or empty, the eight-case table picks up, down or partial." },
  { id: "recuperator", scope: "classroom", reads: ["co2", "classroom_all_stop", "recuperator_auto"], drives: ["recuperator"], doc: "On above co2_on, off below co2_off." },
  { id: "alarm", scope: "classroom", reads: ["classroom_all_stop"], drives: ["buzzer"], doc: "The buzzer sounds while ALL STOP is pressed." },
] as const

export type Role = keyof typeof POINTS
