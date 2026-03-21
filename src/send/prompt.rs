use anyhow::{Context, Result};
use inquire::{Select, Text};
use std::fmt;
use std::process::Command;
use std::time::Duration;

use super::parse::{SendFrame, parse_can_data, parse_can_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendMode {
    Once,
    ManualRepeat,
    CyclicInterval,
    CyclicFrequency,
}

impl fmt::Display for SendMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SendMode::Once => write!(f, "Send once then exit"),
            SendMode::ManualRepeat => write!(f, "Send once each time manually"),
            SendMode::CyclicInterval => write!(f, "Send cyclically with interval"),
            SendMode::CyclicFrequency => write!(f, "Send cyclically with frequency"),
        }
    }
}

pub fn prompt_can_interface() -> Result<String> {
    let mut ifaces = detect_can_interfaces()?;
    ifaces.push("Enter manually".into());

    let choice = Select::new("Select CAN interface", ifaces).prompt()?;

    if choice == "Enter manually" {
        Ok(Text::new("Enter interface name").prompt()?)
    } else {
        Ok(choice)
    }
}

pub fn prompt_frame() -> Result<SendFrame> {
    let id_input = Text::new("Enter CAN ID (hex, e.g. 123):")
        .prompt()
        .context("failed to read CAN ID")?;

    let id = parse_can_id(&id_input)?;

    let data_input = Text::new("Enter data bytes (e.g. 11 22 33 44):")
        .with_default("")
        .prompt()
        .context("failed to read CAN data")?;

    let data = parse_can_data(&data_input)?;

    Ok(SendFrame { id, data })
}

pub fn prompt_send_mode() -> Result<SendMode> {
    Select::new(
        "How do you want to send it?",
        vec![
            SendMode::Once,
            SendMode::ManualRepeat,
            SendMode::CyclicInterval,
            SendMode::CyclicFrequency,
        ],
    )
    .prompt()
    .context("failed to read send mode")
}

pub fn prompt_interval() -> Result<Duration> {
    let input = Text::new("Interval in milliseconds:")
        .with_default("100")
        .prompt()
        .context("failed to read interval")?;

    let millis: u64 = input
        .trim()
        .parse()
        .context("interval must be a valid positive integer")?;

    Ok(Duration::from_millis(millis))
}

pub fn prompt_frequency() -> Result<Duration> {
    let input = Text::new("Frequency in Hz:")
        .with_default("10")
        .prompt()
        .context("failed to read frequency")?;

    let hz: u64 = input
        .trim()
        .parse()
        .context("frequency must be a valid positive integer")?;

    Ok(Duration::from_millis(1000 / hz))
}

fn detect_can_interfaces() -> Result<Vec<String>> {
    let output = Command::new("ip")
        .args(["-brief", "link"])
        .output()
        .context("failed to run 'ip -brief link'")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut interfaces = Vec::new();

    for line in stdout.lines() {
        let iface = line.split_whitespace().next().unwrap_or("");

        if iface.starts_with("can") || iface.starts_with("slcan") || iface.starts_with("vcan") {
            interfaces.push(iface.to_string());
        }
    }

    interfaces.sort();
    Ok(interfaces)
}
