// Derived by fid-topology from topology.toml for firmware `classroom` — do not edit.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Parameter {
    Co2On = 1,
    Co2Off = 2,
    BlindWarm = 3,
    BlindBright = 4,
    BlindPartial = 5,
}

pub const PARAMETERS: [Parameter; 5] = [Parameter::Co2On, Parameter::Co2Off, Parameter::BlindWarm, Parameter::BlindBright, Parameter::BlindPartial];

impl Parameter {
    pub const fn id(self) -> u16 {
        self as u16
    }

    pub fn from_id(id: u16) -> Option<Parameter> {
        PARAMETERS.iter().copied().find(|p| p.id() == id)
    }

    pub const fn default(self) -> f32 {
        match self {
            Parameter::Co2On => 1000.0,
            Parameter::Co2Off => 800.0,
            Parameter::BlindWarm => 24.0,
            Parameter::BlindBright => 20000.0,
            Parameter::BlindPartial => 8.0,
        }
    }

    pub const fn min(self) -> f32 {
        match self {
            Parameter::Co2On => 600.0,
            Parameter::Co2Off => 400.0,
            Parameter::BlindWarm => 15.0,
            Parameter::BlindBright => 1000.0,
            Parameter::BlindPartial => 1.0,
        }
    }

    pub const fn max(self) -> f32 {
        match self {
            Parameter::Co2On => 2000.0,
            Parameter::Co2Off => 1800.0,
            Parameter::BlindWarm => 35.0,
            Parameter::BlindBright => 60000.0,
            Parameter::BlindPartial => 60.0,
        }
    }

    pub const fn unit(self) -> &'static str {
        match self {
            Parameter::Co2On => "ppm",
            Parameter::Co2Off => "ppm",
            Parameter::BlindWarm => "°C",
            Parameter::BlindBright => "lx",
            Parameter::BlindPartial => "s",
        }
    }

    pub fn accepts(self, value: f32) -> bool {
        self.min() <= value && value <= self.max()
    }
}

pub const TIMETABLE_GRANULARITY_MIN: u16 = 5;
pub const TIMETABLE_OCCUPIED_WHEN_MISSING: bool = true;
