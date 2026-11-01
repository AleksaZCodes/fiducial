// Derived by fid-topology from topology.toml — do not edit.

export interface Card {
  readonly point: string
  readonly unit: string
  readonly limits: readonly string[]
}

export const CARDS = {
  "classroom": [
    { point: "co2", unit: "ppm", limits: ["co2_on", "co2_off"] },
    { point: "inside_temp", unit: "°C", limits: [] },
    { point: "humidity", unit: "%", limits: [] },
    { point: "outside_light", unit: "lx", limits: ["blind_bright"] },
    { point: "outside_temp", unit: "°C", limits: ["blind_warm"] },
  ],
  "roof": [
    { point: "soil_1", unit: "%", limits: ["soil_start", "soil_stop"] },
    { point: "soil_2", unit: "%", limits: [] },
    { point: "roof_bare", unit: "°C", limits: [] },
    { point: "roof_green", unit: "°C", limits: [] },
  ],
} as const satisfies Record<string, readonly Card[]>
