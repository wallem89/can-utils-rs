use anyhow::{Context, Result};
use inquire::{Select, Text};
use std::{fs, path::PathBuf};

use crate::setup::models::{
    CanBitrate, ExistingIfaceAction, NativeConfig, SlcanConfig, SlcanSpeed, VirtualConfig,
    slcan_speeds,
};

pub fn list_serial_candidates() -> Result<Vec<String>> {
    let mut devices = Vec::new();

    for entry in fs::read_dir("/dev").context("failed to read /dev")? {
        let entry = entry?;
        let path: PathBuf = entry.path();

        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        if name.starts_with("ttyUSB") || name.starts_with("ttyACM") {
            devices.push(format!("/dev/{name}"));
        }
    }

    devices.sort();
    Ok(devices)
}

pub fn prompt_serial_device() -> Result<String> {
    let mut options = list_serial_candidates()?;

    if options.is_empty() {
        return Ok(
            Text::new("No serial interfaces detected. Enter device manually:")
                .with_default("/dev/ttyUSB0")
                .prompt()?,
        );
    }

    options.push("Enter manually".to_string());

    let selected = Select::new("Select serial device:", options).prompt()?;

    if selected == "Enter manually" {
        Ok(Text::new("Serial device:")
            .with_default("/dev/ttyUSB0")
            .prompt()?)
    } else {
        Ok(selected)
    }
}

pub fn prompt_native() -> Result<NativeConfig> {
    let iface = Text::new("CAN interface name:")
        .with_default("can0")
        .prompt()?;
    let bitrate: CanBitrate =
        Select::new("Select CAN bitrate:", CanBitrate::can_bitrates()).prompt()?;

    Ok(NativeConfig { iface, bitrate })
}

pub fn prompt_slcan() -> Result<SlcanConfig> {
    let tty = prompt_serial_device()?;

    let iface = Text::new("SLCAN interface name:")
        .with_default("slcan0")
        .prompt()?;
    let speed: SlcanSpeed = Select::new("Select CAN bitrate:", slcan_speeds()).prompt()?;

    let uart_baud_str = Select::new(
        "Select UART baud rate to adapter:",
        vec!["115200", "230400", "460800", "921600", "3000000"],
    )
    .with_starting_cursor(4)
    .prompt()?;

    let uart_baud = uart_baud_str.parse::<u32>()?;

    Ok(SlcanConfig {
        tty,
        iface,
        speed,
        uart_baud,
    })
}

pub fn prompt_virtual() -> Result<VirtualConfig> {
    let iface = Text::new("Virtual CAN interface name:")
        .with_default("vcan0")
        .prompt()?;

    Ok(VirtualConfig { iface })
}

pub fn prompt_existing_interface_action(iface: &str) -> Result<ExistingIfaceAction> {
    let choice = Select::new(
        &format!(
            "Interface '{}' already exists. What do you want to do?",
            iface
        ),
        vec![
            "Replace existing interface",
            "Enter another interface name",
            "Keep existing and skip setup",
            "Cancel",
        ],
    )
    .prompt()?;

    Ok(match choice {
        "Replace existing interface" => ExistingIfaceAction::Replace,
        "Enter another interface name" => ExistingIfaceAction::Rename,
        "Keep existing and skip setup" => ExistingIfaceAction::Skip,
        _ => ExistingIfaceAction::Cancel,
    })
}

pub fn prompt_new_interface_name(current: &str) -> Result<String> {
    Ok(Text::new("Enter a new interface name:")
        .with_initial_value(current)
        .prompt()?)
}
