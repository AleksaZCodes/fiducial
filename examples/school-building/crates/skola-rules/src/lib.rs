#![no_std]

use skola_messages::{Output, Reason, Selector};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    pub value: Option<f32>,
}

impl Reading {
    pub const fn ok(v: f32) -> Self {
        Reading { value: Some(v) }
    }

    pub const fn fault() -> Self {
        Reading { value: None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    pub output: Output,
    pub reason: Reason,
}

const fn decide(output: Output, reason: Reason) -> Decision {
    Decision { output, reason }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    Auto,
    Held(Reason),
}

pub fn gate(all_stop: bool, selector: Selector) -> Gate {
    if all_stop {
        return Gate::Held(Reason::AllStop);
    }
    match selector {
        Selector::Auto => Gate::Auto,
        Selector::Manual => Gate::Held(Reason::Manual),
        Selector::Off => Gate::Held(Reason::SelectorOff),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlindLimits {
    pub warm_c: f32,
    pub bright_lx: f32,
}

pub fn blinds(
    gate: Gate,
    outside_c: Reading,
    light_lx: Reading,
    occupied: bool,
    limits: BlindLimits,
    held: Output,
) -> Decision {
    if let Gate::Held(r) = gate {
        return decide(held, r);
    }
    let (Some(t), Some(l)) = (outside_c.value, light_lx.value) else {
        return decide(held, Reason::SensorFault);
    };
    let warm = t > limits.warm_c;
    let bright = l > limits.bright_lx;
    match (warm, bright, occupied) {
        (true, true, false) => decide(Output::Down, Reason::EmptyWarmBright),
        (true, true, true) => decide(Output::Partial, Reason::LessonWarmBright),
        (true, false, false) => decide(Output::Down, Reason::EmptyWarmDark),
        (true, false, true) => decide(Output::Up, Reason::LessonWarmDark),
        (false, true, false) => decide(Output::Up, Reason::EmptyColdBright),
        (false, true, true) => decide(Output::Partial, Reason::LessonColdBright),
        (false, false, false) => decide(Output::Down, Reason::EmptyColdDark),
        (false, false, true) => decide(Output::Down, Reason::LessonColdDark),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    pub on: f32,
    pub off: f32,
}

pub fn recuperator(gate: Gate, co2_ppm: Reading, band: Band, running: bool) -> Decision {
    if let Gate::Held(r) = gate {
        return decide(if running { Output::On } else { Output::Off }, r);
    }
    let Some(c) = co2_ppm.value else {
        return decide(Output::Off, Reason::SensorFault);
    };
    if c > band.on || (running && c >= band.off) {
        decide(Output::On, Reason::Co2High)
    } else {
        decide(Output::Off, Reason::Co2Low)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Water {
    pub pump: Decision,
    pub valve: Decision,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tank {
    pub low_wet: bool,
    pub high_wet: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn irrigation(
    pump_gate: Gate,
    valve_gate: Gate,
    soil: [Reading; 2],
    tank: Tank,
    band: Band,
    pumping: bool,
    refilling: bool,
) -> Water {
    let fault = soil.iter().any(|r| r.value.is_none());
    let average = soil.iter().filter_map(|r| r.value).sum::<f32>() / soil.len() as f32;

    let valve = match valve_gate {
        Gate::Held(r) => decide(if refilling { Output::Open } else { Output::Closed }, r),
        Gate::Auto if tank.high_wet => decide(Output::Closed, Reason::TankFull),
        Gate::Auto if !tank.low_wet || refilling => decide(Output::Open, Reason::TankLow),
        Gate::Auto => decide(Output::Closed, Reason::TankFull),
    };

    let pump = match pump_gate {
        Gate::Held(Reason::AllStop) => decide(Output::Off, Reason::AllStop),
        Gate::Held(r) => decide(if pumping && tank.low_wet { Output::On } else { Output::Off }, r),
        Gate::Auto if fault => decide(Output::Off, Reason::SensorFault),
        Gate::Auto if !tank.low_wet => decide(Output::Off, Reason::TankLow),
        Gate::Auto if average < band.on || (pumping && average < band.off) => {
            decide(Output::On, Reason::SoilDry)
        }
        Gate::Auto => decide(Output::Off, Reason::SoilWet),
    };

    let valve = if fault && valve_gate == Gate::Auto {
        decide(Output::Closed, Reason::SensorFault)
    } else {
        valve
    };
    Water { pump, valve }
}

pub fn command_allowed(gate: Gate) -> Result<(), Reason> {
    match gate {
        Gate::Auto => Ok(()),
        Gate::Held(Reason::AllStop) => Err(Reason::AllStop),
        Gate::Held(_) => Err(Reason::NotInAuto),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: BlindLimits = BlindLimits { warm_c: 24.0, bright_lx: 20_000.0 };
    const CO2: Band = Band { on: 1000.0, off: 800.0 };
    const SOIL: Band = Band { on: 30.0, off: 40.0 };
    const FULL: Tank = Tank { low_wet: true, high_wet: true };
    const MID: Tank = Tank { low_wet: true, high_wet: false };
    const EMPTY: Tank = Tank { low_wet: false, high_wet: false };

    fn blind(t: f32, l: f32, occupied: bool) -> Decision {
        blinds(Gate::Auto, Reading::ok(t), Reading::ok(l), occupied, LIMITS, Output::Stopped)
    }

    #[test]
    fn every_row_of_the_blind_table() {
        let warm = 30.0;
        let cold = 10.0;
        let bright = 50_000.0;
        let dark = 500.0;
        assert_eq!(blind(warm, bright, false), decide(Output::Down, Reason::EmptyWarmBright));
        assert_eq!(blind(warm, bright, true), decide(Output::Partial, Reason::LessonWarmBright));
        assert_eq!(blind(warm, dark, false), decide(Output::Down, Reason::EmptyWarmDark));
        assert_eq!(blind(warm, dark, true), decide(Output::Up, Reason::LessonWarmDark));
        assert_eq!(blind(cold, bright, false), decide(Output::Up, Reason::EmptyColdBright));
        assert_eq!(blind(cold, bright, true), decide(Output::Partial, Reason::LessonColdBright));
        assert_eq!(blind(cold, dark, false), decide(Output::Down, Reason::EmptyColdDark));
        assert_eq!(blind(cold, dark, true), decide(Output::Down, Reason::LessonColdDark));
    }

    #[test]
    fn warm_and_bright_mean_strictly_above_their_limits() {
        assert_eq!(blind(24.0, 20_000.0, true).reason, Reason::LessonColdDark);
        assert_eq!(blind(24.1, 20_001.0, true).reason, Reason::LessonWarmBright);
    }

    #[test]
    fn a_blind_holds_on_a_fault_and_off_auto() {
        let held = blinds(Gate::Auto, Reading::fault(), Reading::ok(1.0), true, LIMITS, Output::Partial);
        assert_eq!(held, decide(Output::Partial, Reason::SensorFault));
        let manual = blinds(gate(false, Selector::Manual), Reading::ok(30.0), Reading::ok(1.0), true, LIMITS, Output::Up);
        assert_eq!(manual, decide(Output::Up, Reason::Manual));
        let stop = blinds(gate(true, Selector::Auto), Reading::ok(30.0), Reading::ok(1.0), true, LIMITS, Output::Up);
        assert_eq!(stop.reason, Reason::AllStop);
    }

    #[test]
    fn the_recuperator_has_both_hysteresis_edges() {
        let r = |c: f32, running: bool| recuperator(Gate::Auto, Reading::ok(c), CO2, running).output;
        assert_eq!(r(1000.0, false), Output::Off);
        assert_eq!(r(1000.1, false), Output::On);
        assert_eq!(r(900.0, true), Output::On);
        assert_eq!(r(800.0, true), Output::On);
        assert_eq!(r(799.9, true), Output::Off);
        assert_eq!(r(900.0, false), Output::Off);
        assert_eq!(recuperator(Gate::Auto, Reading::fault(), CO2, true), decide(Output::Off, Reason::SensorFault));
    }

    fn water(soil: [f32; 2], tank: Tank, pumping: bool, refilling: bool) -> Water {
        irrigation(Gate::Auto, Gate::Auto, [Reading::ok(soil[0]), Reading::ok(soil[1])], tank, SOIL, pumping, refilling)
    }

    #[test]
    fn irrigation_has_both_hysteresis_edges_on_the_average() {
        assert_eq!(water([20.0, 39.0], MID, false, false).pump.output, Output::On);
        assert_eq!(water([30.0, 30.0], MID, false, false).pump.output, Output::Off);
        assert_eq!(water([35.0, 35.0], MID, true, false).pump.output, Output::On);
        assert_eq!(water([40.0, 40.0], MID, true, false).pump.output, Output::Off);
    }

    #[test]
    fn the_pump_never_runs_dry_and_the_tank_refills() {
        let w = water([10.0, 10.0], EMPTY, true, false);
        assert_eq!(w.pump, decide(Output::Off, Reason::TankLow));
        assert_eq!(w.valve, decide(Output::Open, Reason::TankLow));
        let mid = water([10.0, 10.0], MID, false, true);
        assert_eq!(mid.valve.output, Output::Open);
        assert_eq!(mid.pump.output, Output::On);
        let full = water([10.0, 10.0], FULL, true, true);
        assert_eq!(full.valve, decide(Output::Closed, Reason::TankFull));
        assert_eq!(full.pump.output, Output::On);
        let manual = irrigation(gate(false, Selector::Manual), Gate::Auto, [Reading::ok(10.0); 2], EMPTY, SOIL, true, false);
        assert_eq!(manual.pump.output, Output::Off);
    }

    #[test]
    fn a_soil_fault_stops_pump_and_valve() {
        let w = irrigation(Gate::Auto, Gate::Auto, [Reading::ok(10.0), Reading::fault()], MID, SOIL, true, true);
        assert_eq!(w.pump, decide(Output::Off, Reason::SensorFault));
        assert_eq!(w.valve, decide(Output::Closed, Reason::SensorFault));
    }

    #[test]
    fn a_command_is_refused_unless_in_auto() {
        assert_eq!(command_allowed(gate(false, Selector::Auto)), Ok(()));
        assert_eq!(command_allowed(gate(false, Selector::Manual)), Err(Reason::NotInAuto));
        assert_eq!(command_allowed(gate(true, Selector::Auto)), Err(Reason::AllStop));
    }
}
