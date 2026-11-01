#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ts")]
use ts_rs::TS;

#[path = "generated/topics.rs"]
pub mod topics;

pub type PointId = u16;
pub type DeviceId = u16;
pub type ParameterId = u16;
pub type CommandId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export, rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    Boot,
    AllStop,
    Manual,
    SelectorOff,
    NotInAuto,
    Unsafe,
    SensorFault,
    NoTimetable,
    LessonWarmBright,
    LessonWarmDark,
    LessonColdBright,
    LessonColdDark,
    EmptyWarmBright,
    EmptyWarmDark,
    EmptyColdBright,
    EmptyColdDark,
    Co2High,
    Co2Low,
    SoilDry,
    SoilWet,
    TankLow,
    TankFull,
    Command,
}

impl Reason {
    pub const ALL: [Reason; 23] = [
        Reason::Boot,
        Reason::AllStop,
        Reason::Manual,
        Reason::SelectorOff,
        Reason::NotInAuto,
        Reason::Unsafe,
        Reason::SensorFault,
        Reason::NoTimetable,
        Reason::LessonWarmBright,
        Reason::LessonWarmDark,
        Reason::LessonColdBright,
        Reason::LessonColdDark,
        Reason::EmptyWarmBright,
        Reason::EmptyWarmDark,
        Reason::EmptyColdBright,
        Reason::EmptyColdDark,
        Reason::Co2High,
        Reason::Co2Low,
        Reason::SoilDry,
        Reason::SoilWet,
        Reason::TankLow,
        Reason::TankFull,
        Reason::Command,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Reason::Boot => "boot",
            Reason::AllStop => "all_stop",
            Reason::Manual => "manual",
            Reason::SelectorOff => "selector_off",
            Reason::NotInAuto => "not_in_auto",
            Reason::Unsafe => "unsafe",
            Reason::SensorFault => "sensor_fault",
            Reason::NoTimetable => "no_timetable",
            Reason::LessonWarmBright => "lesson_warm_bright",
            Reason::LessonWarmDark => "lesson_warm_dark",
            Reason::LessonColdBright => "lesson_cold_bright",
            Reason::LessonColdDark => "lesson_cold_dark",
            Reason::EmptyWarmBright => "empty_warm_bright",
            Reason::EmptyWarmDark => "empty_warm_dark",
            Reason::EmptyColdBright => "empty_cold_bright",
            Reason::EmptyColdDark => "empty_cold_dark",
            Reason::Co2High => "co2_high",
            Reason::Co2Low => "co2_low",
            Reason::SoilDry => "soil_dry",
            Reason::SoilWet => "soil_wet",
            Reason::TankLow => "tank_low",
            Reason::TankFull => "tank_full",
            Reason::Command => "command",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export, rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum Selector {
    Manual,
    Off,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export, rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum Output {
    Off,
    On,
    Up,
    Down,
    Partial,
    Stopped,
    Open,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export, rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum Action {
    On,
    Off,
    Up,
    Down,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Reading {
    pub point: PointId,
    pub value: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct State {
    pub device: DeviceId,
    pub output: Output,
    pub selector: Selector,
    pub reason: Reason,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Status {
    pub uptime_s: u32,
    pub readings: Vec<Reading>,
    pub states: Vec<State>,
    pub alarms: Vec<Alarm>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Command {
    pub id: CommandId,
    pub device: DeviceId,
    pub action: Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export, rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Executed,
    Refused(Reason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct CommandAck {
    pub id: CommandId,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Threshold {
    pub parameter: ParameterId,
    pub value: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Thresholds {
    pub values: Vec<Threshold>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Slot {
    pub weekday: u8,
    pub from_min: u16,
    pub to_min: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Timetable {
    pub slots: Vec<Slot>,
}

impl Timetable {
    pub fn occupied(&self, weekday: u8, minute: u16) -> bool {
        self.slots
            .iter()
            .any(|s| s.weekday == weekday && s.from_min <= minute && minute < s.to_min)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export, rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum AlarmKind {
    NoWater,
    SensorFault,
    Offline,
    AllStop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Alarm {
    pub kind: AlarmKind,
    pub source: Option<PointId>,
}

pub fn encode<T: Serialize>(message: &T) -> postcard::Result<Vec<u8>> {
    postcard::to_allocvec(message)
}

pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> postcard::Result<T> {
    postcard::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn every_message_round_trips_through_postcard() {
        let status = Status {
            uptime_s: 42,
            readings: vec![Reading { point: 1, value: Some(812.0) }, Reading { point: 2, value: None }],
            states: vec![State { device: 3, output: Output::Partial, selector: Selector::Auto, reason: Reason::LessonWarmBright }],
            alarms: vec![Alarm { kind: AlarmKind::SensorFault, source: Some(2) }],
        };
        assert_eq!(decode::<Status>(&encode(&status).unwrap()).unwrap(), status);
        let ack = CommandAck { id: 7, outcome: Outcome::Refused(Reason::NotInAuto) };
        assert_eq!(decode::<CommandAck>(&encode(&ack).unwrap()).unwrap(), ack);
        let t = Timetable { slots: vec![Slot { weekday: 1, from_min: 480, to_min: 525 }] };
        assert_eq!(decode::<Timetable>(&encode(&t).unwrap()).unwrap(), t);
    }

    #[test]
    fn the_web_boundary_is_json_with_snake_case_codes() {
        let ack = CommandAck { id: 7, outcome: Outcome::Refused(Reason::NotInAuto) };
        assert_eq!(
            serde_json::to_string(&ack).unwrap(),
            r#"{"id":7,"outcome":{"refused":"not_in_auto"}}"#
        );
    }

    #[test]
    fn every_reason_code_is_unique_and_matches_its_serde_name() {
        for r in Reason::ALL {
            assert_eq!(serde_json::to_string(&r).unwrap(), alloc::format!("\"{}\"", r.code()));
        }
        let mut codes: Vec<&str> = Reason::ALL.iter().map(|r| r.code()).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), Reason::ALL.len());
    }

    #[cfg(feature = "std")]
    #[test]
    fn every_reason_has_a_message_in_every_catalog() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../messages");
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let catalog: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            for r in Reason::ALL {
                assert!(
                    catalog["reason"][r.code()].is_string(),
                    "{} has no reason.{}",
                    path.display(),
                    r.code()
                );
            }
        }
    }
}
