// Derived by fid-topology from topology.toml for firmware `roof` — do not edit.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Parameter {
    SoilStart = 1,
    SoilStop = 2,
}

pub const PARAMETERS: [Parameter; 2] = [Parameter::SoilStart, Parameter::SoilStop];

impl Parameter {
    pub const fn id(self) -> u16 {
        self as u16
    }

    pub fn from_id(id: u16) -> Option<Parameter> {
        PARAMETERS.iter().copied().find(|p| p.id() == id)
    }

    pub const fn default(self) -> f32 {
        match self {
            Parameter::SoilStart => 30.0,
            Parameter::SoilStop => 40.0,
        }
    }

    pub const fn min(self) -> f32 {
        match self {
            Parameter::SoilStart => 10.0,
            Parameter::SoilStop => 20.0,
        }
    }

    pub const fn max(self) -> f32 {
        match self {
            Parameter::SoilStart => 60.0,
            Parameter::SoilStop => 80.0,
        }
    }

    pub const fn unit(self) -> &'static str {
        match self {
            Parameter::SoilStart => "%",
            Parameter::SoilStop => "%",
        }
    }

    pub fn accepts(self, value: f32) -> bool {
        self.min() <= value && value <= self.max()
    }
}
