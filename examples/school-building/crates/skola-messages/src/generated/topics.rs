// Derived by fid-topology from topology.toml — do not edit.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topic {
    Stanje,
    Nalog,
    Potvrda,
    Pragovi,
    Raspored,
    Online,
}

pub const TOPICS: [Topic; 6] = [Topic::Stanje, Topic::Nalog, Topic::Potvrda, Topic::Pragovi, Topic::Raspored, Topic::Online];

impl Topic {
    pub const fn suffix(self) -> &'static str {
        match self {
            Topic::Stanje => "stanje",
            Topic::Nalog => "nalog",
            Topic::Potvrda => "potvrda",
            Topic::Pragovi => "pragovi",
            Topic::Raspored => "raspored",
            Topic::Online => "online",
        }
    }

    pub const fn retained(self) -> bool {
        matches!(self, Topic::Pragovi | Topic::Raspored)
    }

    pub const fn from_device(self) -> bool {
        matches!(self, Topic::Stanje | Topic::Potvrda)
    }
}

pub struct NodeInfo {
    pub id: &'static str,
    pub system: &'static str,
    pub firmware: Option<&'static str>,
}

pub const NODES: [NodeInfo; 3] = [
    NodeInfo { id: "roof", system: "skola", firmware: Some("roof") },
    NodeInfo { id: "classroom_1", system: "skola", firmware: Some("classroom") },
    NodeInfo { id: "server", system: "skola", firmware: None },
];

#[cfg(feature = "std")]
pub fn topic(system: &str, node: &str, kind: Topic) -> std::string::String {
    std::format!("{system}/{node}/{}", kind.suffix())
}
