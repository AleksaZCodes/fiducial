// Derived by fid-topology from topology.toml for firmware `classroom` — do not edit.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Point {
    Co2 = 1,
    InsideTemp = 2,
    Humidity = 3,
    OutsideLight = 4,
    OutsideTemp = 5,
    ClassroomAllStop = 6,
    BlindAuto = 7,
    RecuperatorAuto = 8,
    BlindUp = 9,
    BlindDown = 10,
    Recuperator = 11,
    Buzzer = 12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    Bus { bus: &'static str, address: u32 },
    Adc { channel: &'static str },
    Pin { pin: &'static str },
    Net { net: &'static str },
}

pub const POINTS: [Point; 12] = [Point::Co2, Point::InsideTemp, Point::Humidity, Point::OutsideLight, Point::OutsideTemp, Point::ClassroomAllStop, Point::BlindAuto, Point::RecuperatorAuto, Point::BlindUp, Point::BlindDown, Point::Recuperator, Point::Buzzer];

impl Point {
    pub const fn id(self) -> u16 {
        self as u16
    }

    pub const fn name(self) -> &'static str {
        match self {
            Point::Co2 => "co2",
            Point::InsideTemp => "inside_temp",
            Point::Humidity => "humidity",
            Point::OutsideLight => "outside_light",
            Point::OutsideTemp => "outside_temp",
            Point::ClassroomAllStop => "classroom_all_stop",
            Point::BlindAuto => "blind_auto",
            Point::RecuperatorAuto => "recuperator_auto",
            Point::BlindUp => "blind_up",
            Point::BlindDown => "blind_down",
            Point::Recuperator => "recuperator",
            Point::Buzzer => "buzzer",
        }
    }

    pub const fn unit(self) -> Option<&'static str> {
        match self {
            Point::Co2 => Some("ppm"),
            Point::InsideTemp => Some("°C"),
            Point::Humidity => Some("%"),
            Point::OutsideLight => Some("lx"),
            Point::OutsideTemp => Some("°C"),
            Point::ClassroomAllStop => None,
            Point::BlindAuto => None,
            Point::RecuperatorAuto => None,
            Point::BlindUp => None,
            Point::BlindDown => None,
            Point::Recuperator => None,
            Point::Buzzer => None,
        }
    }

    pub const fn range(self) -> Option<(f32, f32)> {
        match self {
            Point::Co2 => Some((400.0, 5000.0)),
            Point::InsideTemp => Some((-10.0, 60.0)),
            Point::Humidity => Some((0.0, 100.0)),
            Point::OutsideLight => Some((0.0, 65535.0)),
            Point::OutsideTemp => Some((-30.0, 60.0)),
            Point::ClassroomAllStop => None,
            Point::BlindAuto => None,
            Point::RecuperatorAuto => None,
            Point::BlindUp => None,
            Point::BlindDown => None,
            Point::Recuperator => None,
            Point::Buzzer => None,
        }
    }

    pub const fn binding(self) -> Binding {
        match self {
            Point::Co2 => Binding::Bus { bus: "classroom_i2c", address: 0x62 },
            Point::InsideTemp => Binding::Bus { bus: "classroom_i2c", address: 0x62 },
            Point::Humidity => Binding::Bus { bus: "classroom_i2c", address: 0x62 },
            Point::OutsideLight => Binding::Bus { bus: "classroom_i2c", address: 0x23 },
            Point::OutsideTemp => Binding::Bus { bus: "classroom_1wire", address: 0 },
            Point::ClassroomAllStop => Binding::Pin { pin: "GPIO2" },
            Point::BlindAuto => Binding::Pin { pin: "GPIO22" },
            Point::RecuperatorAuto => Binding::Pin { pin: "GPIO23" },
            Point::BlindUp => Binding::Pin { pin: "GPIO18" },
            Point::BlindDown => Binding::Pin { pin: "GPIO19" },
            Point::Recuperator => Binding::Pin { pin: "GPIO20" },
            Point::Buzzer => Binding::Pin { pin: "GPIO21" },
        }
    }

    pub const fn calibrated(self) -> bool {
        match self {
            Point::Co2 => false,
            Point::InsideTemp => false,
            Point::Humidity => false,
            Point::OutsideLight => false,
            Point::OutsideTemp => false,
            Point::ClassroomAllStop => false,
            Point::BlindAuto => false,
            Point::RecuperatorAuto => false,
            Point::BlindUp => false,
            Point::BlindDown => false,
            Point::Recuperator => false,
            Point::Buzzer => false,
        }
    }

    pub const fn interlock(self) -> Option<Point> {
        match self {
            Point::Co2 => None,
            Point::InsideTemp => None,
            Point::Humidity => None,
            Point::OutsideLight => None,
            Point::OutsideTemp => None,
            Point::ClassroomAllStop => None,
            Point::BlindAuto => None,
            Point::RecuperatorAuto => None,
            Point::BlindUp => Some(Point::BlindDown),
            Point::BlindDown => Some(Point::BlindUp),
            Point::Recuperator => None,
            Point::Buzzer => None,
        }
    }
}
