// Derived by fid-topology from topology.toml for firmware `roof` — do not edit.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Point {
    Soil1 = 1,
    Soil2 = 2,
    RoofBare = 3,
    RoofGreen = 4,
    FloatLow = 5,
    FloatHigh = 6,
    RoofAllStop = 7,
    PumpAuto = 8,
    ValveAuto = 9,
    Pump = 10,
    Valve = 11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    Bus { bus: &'static str, address: u32 },
    Adc { channel: &'static str },
    Pin { pin: &'static str },
    Net { net: &'static str },
}

pub const POINTS: [Point; 11] = [Point::Soil1, Point::Soil2, Point::RoofBare, Point::RoofGreen, Point::FloatLow, Point::FloatHigh, Point::RoofAllStop, Point::PumpAuto, Point::ValveAuto, Point::Pump, Point::Valve];

impl Point {
    pub const fn id(self) -> u16 {
        self as u16
    }

    pub const fn name(self) -> &'static str {
        match self {
            Point::Soil1 => "soil_1",
            Point::Soil2 => "soil_2",
            Point::RoofBare => "roof_bare",
            Point::RoofGreen => "roof_green",
            Point::FloatLow => "float_low",
            Point::FloatHigh => "float_high",
            Point::RoofAllStop => "roof_all_stop",
            Point::PumpAuto => "pump_auto",
            Point::ValveAuto => "valve_auto",
            Point::Pump => "pump",
            Point::Valve => "valve",
        }
    }

    pub const fn unit(self) -> Option<&'static str> {
        match self {
            Point::Soil1 => Some("%"),
            Point::Soil2 => Some("%"),
            Point::RoofBare => Some("°C"),
            Point::RoofGreen => Some("°C"),
            Point::FloatLow => None,
            Point::FloatHigh => None,
            Point::RoofAllStop => None,
            Point::PumpAuto => None,
            Point::ValveAuto => None,
            Point::Pump => None,
            Point::Valve => None,
        }
    }

    pub const fn range(self) -> Option<(f32, f32)> {
        match self {
            Point::Soil1 => Some((0.0, 100.0)),
            Point::Soil2 => Some((0.0, 100.0)),
            Point::RoofBare => Some((-30.0, 90.0)),
            Point::RoofGreen => Some((-30.0, 90.0)),
            Point::FloatLow => None,
            Point::FloatHigh => None,
            Point::RoofAllStop => None,
            Point::PumpAuto => None,
            Point::ValveAuto => None,
            Point::Pump => None,
            Point::Valve => None,
        }
    }

    pub const fn binding(self) -> Binding {
        match self {
            Point::Soil1 => Binding::Net { net: "SOIL_1" },
            Point::Soil2 => Binding::Net { net: "SOIL_2" },
            Point::RoofBare => Binding::Bus { bus: "roof_1wire", address: 0 },
            Point::RoofGreen => Binding::Bus { bus: "roof_1wire", address: 0 },
            Point::FloatLow => Binding::Net { net: "FLOAT_LOW" },
            Point::FloatHigh => Binding::Net { net: "FLOAT_HIGH" },
            Point::RoofAllStop => Binding::Net { net: "ALL_STOP" },
            Point::PumpAuto => Binding::Net { net: "PUMP_AUTO" },
            Point::ValveAuto => Binding::Net { net: "VALVE_AUTO" },
            Point::Pump => Binding::Net { net: "RELAY_PUMP" },
            Point::Valve => Binding::Net { net: "RELAY_VALVE" },
        }
    }

    pub const fn calibrated(self) -> bool {
        match self {
            Point::Soil1 => true,
            Point::Soil2 => true,
            Point::RoofBare => false,
            Point::RoofGreen => false,
            Point::FloatLow => false,
            Point::FloatHigh => false,
            Point::RoofAllStop => false,
            Point::PumpAuto => false,
            Point::ValveAuto => false,
            Point::Pump => false,
            Point::Valve => false,
        }
    }

    pub const fn interlock(self) -> Option<Point> {
        match self {
            Point::Soil1 => None,
            Point::Soil2 => None,
            Point::RoofBare => None,
            Point::RoofGreen => None,
            Point::FloatLow => None,
            Point::FloatHigh => None,
            Point::RoofAllStop => None,
            Point::PumpAuto => None,
            Point::ValveAuto => None,
            Point::Pump => None,
            Point::Valve => None,
        }
    }
}
