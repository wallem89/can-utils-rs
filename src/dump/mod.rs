use anyhow::Result;
use inquire::{Select, Text};
use owo_colors::OwoColorize;
use std::process::Command;

mod format;
mod live;

pub fn run_dump(iface: &str) -> anyhow::Result<()> {
    println!("{} {}", "Pretty CAN Dump on".bold(), iface.cyan().bold());

    println!("{}", "Press Ctrl+C to stop.".yellow());

    live::dump_raw(iface)
}

pub fn run_dump_wizard() -> Result<()> {
    let mut ifaces = detect_can_interfaces()?;

    ifaces.push("Enter manually".into());

    let choice = Select::new("Select CAN interface to dump:", ifaces).prompt()?;

    let iface = if choice == "Enter manually" {
        Text::new("Enter interface name").prompt()?
    } else {
        choice
    };

    println!("{} {}", "Pretty CAN Dump on".bold(), iface.cyan().bold());

    println!("{}", "Press Ctrl+C to stop.".yellow());

    live::dump_raw(&iface)?;

    Ok(())
}

fn detect_can_interfaces() -> Result<Vec<String>> {
    let output = Command::new("ip").args(["-brief", "link"]).output()?;

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
