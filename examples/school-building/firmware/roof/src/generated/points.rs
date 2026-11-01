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
            Point::Soil1 => Binding::Adc { channel: "GPIO4" },
            Point::Soil2 => Binding::Adc { channel: "GPIO5" },
            Point::RoofBare => Binding::Bus { bus: "roof_1wire", address: 0 },
            Point::RoofGreen => Binding::Bus { bus: "roof_1wire", address: 0 },
            Point::FloatLow => Binding::Pin { pin: "GPIO10" },
            Point::FloatHigh => Binding::Pin { pin: "GPIO11" },
            Point::RoofAllStop => Binding::Pin { pin: "GPIO2" },
            Point::PumpAuto => Binding::Pin { pin: "GPIO22" },
            Point::ValveAuto => Binding::Pin { pin: "GPIO23" },
            Point::Pump => Binding::Pin { pin: "GPIO18" },
            Point::Valve => Binding::Pin { pin: "GPIO19" },
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
