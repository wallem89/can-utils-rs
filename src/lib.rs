//! # can-utils-rs
//!
//! `can-utils-rs` is a Rust library and CLI utility for working with
//! **Linux SocketCAN interfaces**. It provides an interactive workflow
//! for creating CAN interfaces and inspecting CAN traffic with a
//! colorized CAN dump.
//!
//! The tool currently supports:
//!
//! - Creating and managing Linux CAN interfaces
//! - Automatic installation of required system prerequisites
//! - Pretty, colorized CAN frame dumping
//!
//! The goal of this project is to provide a **friendlier and more visual
//! alternative to common Linux CAN tools** such as `candump`, while still
//! integrating seamlessly with the standard SocketCAN ecosystem.
//!
//! ---
//!
//! # Features
//!
//! ## CAN Interface Setup
//!
//! The interactive wizard helps configure CAN interfaces without needing
//! to remember the exact Linux commands.
//!
//! Supported interface types:
//!
//! - **Native CAN** (`can0`, `can1`, …)
//! - **SLCAN serial adapters** (`slcan0` via `slcand`)
//! - **Virtual CAN** (`vcan0`) for development and simulation
//!
//! The tool previews the commands before executing them so users can see
//! exactly what will happen.
//!
//! Example native CAN configuration:
//!
//! ```text
//! sudo ip link set can0 up type can bitrate 500000
//! ```
//!
//! ---
//!
//! ## Pretty CAN Dump
//!
//! The project includes a **colorized CAN dump utility** similar to
//! Linux `candump`, but optimized for readability.
//!
//! Example output:
//!
//! ```text
//! (1773430351.545039) vcan0 7FF#00 11 22 33 44 55 66
//! ```
//!
//! Enhancements include:
//!
//! - colored timestamps
//! - highlighted interface name
//! - colored CAN IDs
//! - **per-byte coloring of payload data**
//!
//! This makes it significantly easier to visually scan CAN traffic.
//!
//! ---
//!
//! ## Automatic Prerequisite Detection
//!
//! On startup the tool checks whether required system utilities exist.
//!
//! Missing prerequisites are detected automatically and the user is
//! offered an option to install them.
//!
//! Example prompt:
//!
//! ```text
//! Missing prerequisites:
//!   - can-utils / slcand
//!
//! ? Some required tools are missing. What do you want to do?
//! ❯ Install prerequisites
//!   Continue anyway
//!   Exit
//! ```
//!
//! Installation currently supports **APT-based systems** (Debian / Ubuntu).
//!
//! ---
//!
//! # Supported CAN Interface Types
//!
//! ## Native CAN
//!
//! Configures a hardware CAN controller via SocketCAN.
//!
//! ```text
//! sudo ip link set can0 up type can bitrate 500000
//! ```
//!
//! Typical hardware:
//!
//! - PCI CAN adapters
//! - USB SocketCAN adapters
//! - Raspberry Pi CAN HATs
//! - embedded CAN controllers
//!
//! ---
//!
//! ## SLCAN (Serial CAN)
//!
//! Serial CAN adapters using the `slcand` daemon.
//!
//! Example command sequence:
//!
//! ```text
//! sudo slcand -c -o -f -s6 -t hw -S 3000000 /dev/ttyUSB0 slcan0
//! sudo ip link set up slcan0
//! ```
//!
//! ---
//!
//! ## Virtual CAN
//!
//! Virtual CAN interfaces are extremely useful for development,
//! testing, and CI environments.
//!
//! ```text
//! sudo ip link add dev vcan0 type vcan
//! sudo ip link set up vcan0
//! ```
//!
//! ---
//!
//! # Installation
//!
//! Install the CLI locally with Cargo:
//!
//! ```text
//! cargo install --path .
//! ```
//!
//! Or install it from crates.io without needing to build from source:
//! ```text
//! cargo install can-utils-rs
//! ```
//!
//! Then run:
//!
//! ```text
//! can-utils-rs
//! ```
//!
//! ---
//!
//! # Demo
//! ![can-utils-rs demo](demo/setup-and-dump.gif)
//!
//! ---
//!
//! # Usage
//!
//! Running the binary launches an interactive menu.
//!
//! ```text
//! $ can-utils-rs
//!
//! ? What do you want to do?
//! ❯ Create or manage a CAN interface
//!   Start pretty CAN dump
//!   Create/manage CAN interface then start dump
//! ```
//!
//! The setup workflow asks for:
//!
//! - interface type
//! - interface name
//! - CAN bitrate
//! - serial device (for SLCAN)
//!
//! Before applying changes the tool prints the commands that will run.
//!
//! ---
//!
//! # Interface Safety
//!
//! If an interface already exists the user is asked how to proceed:
//!
//! ```text
//! Interface 'can0' already exists.
//!
//! ❯ Replace existing interface
//!   Enter another interface name
//!   Keep existing and skip setup
//!   Cancel
//! ```
//!
//! When replacing an interface the tool will:
//!
//! - bring the interface down
//! - remove it if necessary (`vcan` or `slcan`)
//! - recreate the interface with the selected configuration
//!
//! ---
//!
//! # Linux Requirements
//!
//! The following system packages are required:
//!
//! ```text
//! iproute2
//! can-utils
//! kmod
//! ```
//!
//! For virtual CAN interfaces:
//!
//! ```text
//! sudo modprobe vcan
//! ```
//!
//! The tool can optionally install these automatically on supported systems.
//!
//! ---
//!
//! # Design
//!
//! The project is structured into modular subsystems:
//!
//! ```text
//! setup::models   - shared configuration types
//! setup::prompt   - interactive setup wizard
//! setup::plan     - command planning / preview generation
//! setup::exec     - command execution helpers
//! setup::prereqs  - prerequisite detection and installation
//!
//! dump::format    - colored frame formatting
//! dump::live      - live SocketCAN frame reader
//! dump::mod       - dump orchestration and interface selection
//! ```
//!
//! This architecture keeps setup logic, CAN traffic handling, and CLI
//! interaction clearly separated.
//!
//! ---
//!
//! # License
//!
//! MIT
//!
//! ---
use anyhow::Result;
use inquire::Select;
use std::fmt;

pub use setup::models::{
    AppConfig, CanBitrate, CanMode, ExistingIfaceAction, InterfaceResolution, NativeConfig,
    SlcanConfig, SlcanSpeed, VirtualConfig,
};
pub mod dump;
pub mod setup;

#[derive(Debug, Clone, Copy)]
enum ToolAction {
    Setup,
    Dump,
    SetupAndDump,
}

impl fmt::Display for ToolAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolAction::Setup => write!(f, "Create or manage a CAN interface"),
            ToolAction::Dump => write!(f, "Start pretty CAN dump"),
            ToolAction::SetupAndDump => {
                write!(f, "Create/manage CAN interface then start dump")
            }
        }
    }
}

pub fn run_interactive() -> Result<()> {
    let action = Select::new(
        "What do you want to do?",
        vec![
            ToolAction::Setup,
            ToolAction::Dump,
            ToolAction::SetupAndDump,
        ],
    )
    .prompt()?;

    match action {
        ToolAction::Setup => {
            setup::run_setup()?;
        }

        ToolAction::Dump => {
            dump::run_dump_wizard()?;
        }

        ToolAction::SetupAndDump => {
            if let Some(config) = setup::run_setup_and_return_config()? {
                dump::run_dump(config.iface())?;
            }
        }
    }

    Ok(())
}
