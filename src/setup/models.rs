use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanBitrate {
    B10K = 10_000,
    B20K = 20_000,
    B50K = 50_000,
    B100K = 100_000,
    B125K = 125_000,
    B250K = 250_000,
    B500K = 500_000,
    B1M = 1_000_000,
}

impl fmt::Display for CanBitrate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            CanBitrate::B1M => "1 Mbit".to_string(),
            CanBitrate::B500K => "500 kbit".to_string(),
            CanBitrate::B250K => "250 kbit".to_string(),
            CanBitrate::B125K => "125 kbit".to_string(),
            CanBitrate::B100K => "100 kbit".to_string(),
            CanBitrate::B50K => "50 kbit".to_string(),
            CanBitrate::B20K => "20 kbit".to_string(),
            CanBitrate::B10K => "10 kbit".to_string(),
        };

        write!(f, "{} ({})", label, *self as u32)
    }
}

impl CanBitrate {
    pub fn bitrate(&self) -> u32 {
        *self as u32
    }
    pub fn can_bitrates() -> Vec<CanBitrate> {
        let mut bitrates = vec![
            CanBitrate::B10K,
            CanBitrate::B20K,
            CanBitrate::B50K,
            CanBitrate::B100K,
            CanBitrate::B125K,
            CanBitrate::B250K,
            CanBitrate::B500K,
            CanBitrate::B1M,
        ];

        bitrates.sort_by_key(|b| std::cmp::Reverse(*b as u32));
        bitrates
    }
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

impl NativeConfig {
    pub fn new(iface: String, bitrate: CanBitrate) -> Self {
        Self { iface, bitrate }
    }
}

#[derive(Debug, Clone)]
pub struct SlcanConfig {
    pub tty: String,
    pub iface: String,
    pub speed: SlcanSpeed,
    pub uart_baud: u32,
}

impl SlcanConfig {
    pub fn new(tty: String, iface: String, speed: SlcanSpeed, uart_baud: u32) -> Self {
        Self {
            tty,
            iface,
            speed,
            uart_baud,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VirtualConfig {
    pub iface: String,
}

impl VirtualConfig {
    pub fn new(iface: String) -> Self {
        Self { iface }
    }
}

#[derive(Debug, Clone)]
pub enum CanConfig {
    Native(NativeConfig),
    Slcan(SlcanConfig),
    Virtual(VirtualConfig),
}

impl CanConfig {
    pub fn new(
        mode: CanMode,
        iface: String,
        bitrate: Option<CanBitrate>,
        slcan_speed: Option<SlcanSpeed>,
        uart_baud: Option<u32>,
    ) -> Self {
        match mode {
            CanMode::Native => CanConfig::Native(NativeConfig {
                iface,
                bitrate: bitrate.expect("Bitrate is required for Native mode"),
            }),
            CanMode::Slcan => CanConfig::Slcan(SlcanConfig {
                tty: "/dev/ttyUSB0".to_string(), // Default value, can be modified later
                iface,
                speed: slcan_speed.expect("SLCAN speed is required for Slcan mode"),
                uart_baud: uart_baud.expect("UART baud rate is required for Slcan mode"),
            }),
            CanMode::Virtual => CanConfig::Virtual(VirtualConfig { iface }),
        }
    }
    pub fn iface(&self) -> &str {
        match self {
            CanConfig::Native(cfg) => &cfg.iface,
            CanConfig::Slcan(cfg) => &cfg.iface,
            CanConfig::Virtual(cfg) => &cfg.iface,
        }
    }

    pub fn set_iface(&mut self, new_iface: String) {
        match self {
            CanConfig::Native(cfg) => cfg.iface = new_iface,
            CanConfig::Slcan(cfg) => cfg.iface = new_iface,
            CanConfig::Virtual(cfg) => cfg.iface = new_iface,
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
