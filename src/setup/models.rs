use std::fmt;

#[derive(Debug, Clone)]
pub struct CanBitrate {
    pub bitrate: u32,
}

impl fmt::Display for CanBitrate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self.bitrate {
            1_000_000 => "1 Mbit".to_string(),
            _ if self.bitrate >= 1_000 => format!("{} kbit", self.bitrate / 1_000),
            _ => self.bitrate.to_string(),
        };

        write!(f, "{:<10} ({})", label, self.bitrate)
    }
}

pub fn can_bitrates() -> Vec<CanBitrate> {
    let mut bitrates = vec![
        CanBitrate { bitrate: 10_000 },
        CanBitrate { bitrate: 20_000 },
        CanBitrate { bitrate: 50_000 },
        CanBitrate { bitrate: 125_000 },
        CanBitrate { bitrate: 250_000 },
        CanBitrate { bitrate: 500_000 },
        CanBitrate { bitrate: 1_000_000 },
    ];

    bitrates.sort_by(|a, b| b.bitrate.cmp(&a.bitrate));
    bitrates
}

#[derive(Debug, Clone)]
pub struct SlcanSpeed {
    pub bitrate: u32,
    pub flag: &'static str,
}

impl fmt::Display for SlcanSpeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self.bitrate {
            1_000_000 => "1 Mbit".to_string(),
            _ if self.bitrate >= 1_000 => format!("{} kbit", self.bitrate / 1_000),
            _ => self.bitrate.to_string(),
        };

        write!(f, "{:<10} ({}, {})", label, self.bitrate, self.flag)
    }
}

pub fn slcan_speeds() -> Vec<SlcanSpeed> {
    let mut speeds = vec![
        SlcanSpeed {
            bitrate: 10_000,
            flag: "s0",
        },
        SlcanSpeed {
            bitrate: 20_000,
            flag: "s1",
        },
        SlcanSpeed {
            bitrate: 50_000,
            flag: "s2",
        },
        SlcanSpeed {
            bitrate: 100_000,
            flag: "s3",
        },
        SlcanSpeed {
            bitrate: 125_000,
            flag: "s4",
        },
        SlcanSpeed {
            bitrate: 250_000,
            flag: "s5",
        },
        SlcanSpeed {
            bitrate: 500_000,
            flag: "s6",
        },
        SlcanSpeed {
            bitrate: 800_000,
            flag: "s7",
        },
        SlcanSpeed {
            bitrate: 1_000_000,
            flag: "s8",
        },
    ];

    speeds.sort_by(|a, b| b.bitrate.cmp(&a.bitrate));
    speeds
}

#[derive(Debug, Clone)]
pub enum CanMode {
    Native,
    Slcan,
    Virtual,
}

impl fmt::Display for CanMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CanMode::Native => write!(f, "Native CAN bus"),
            CanMode::Slcan => write!(f, "Non-native CAN bus (slcand)"),
            CanMode::Virtual => write!(f, "Virtual CAN bus (vcan)"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NativeConfig {
    pub iface: String,
    pub bitrate: CanBitrate,
}

#[derive(Debug, Clone)]
pub struct SlcanConfig {
    pub tty: String,
    pub iface: String,
    pub speed: SlcanSpeed,
    pub uart_baud: u32,
}

#[derive(Debug, Clone)]
pub struct VirtualConfig {
    pub iface: String,
}

#[derive(Debug, Clone)]
pub enum AppConfig {
    Native(NativeConfig),
    Slcan(SlcanConfig),
    Virtual(VirtualConfig),
}

impl AppConfig {
    pub fn iface(&self) -> &str {
        match self {
            AppConfig::Native(cfg) => &cfg.iface,
            AppConfig::Slcan(cfg) => &cfg.iface,
            AppConfig::Virtual(cfg) => &cfg.iface,
        }
    }

    pub fn set_iface(&mut self, new_iface: String) {
        match self {
            AppConfig::Native(cfg) => cfg.iface = new_iface,
            AppConfig::Slcan(cfg) => cfg.iface = new_iface,
            AppConfig::Virtual(cfg) => cfg.iface = new_iface,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExistingIfaceAction {
    Replace,
    Rename,
    Skip,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceResolution {
    Proceed,
    SkipSetup,
}
