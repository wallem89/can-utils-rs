use anyhow::{Context, Result};
use inquire::Select;

pub mod exec;
pub mod models;
pub mod plan;
pub mod prereqs;
pub mod prompt;

use exec::{check_interface_already_available, ensure_interface_name_is_available, execute_config};
use models::{CanConfig, CanMode, InterfaceResolution};
use plan::print_plan;
use prompt::{prompt_native, prompt_slcan, prompt_virtual};

/// Set up a CAN interface without a CLI based on the provided configuration, handling existing interfaces as needed.
pub fn setup(mut config: CanConfig) -> Result<()> {
    if check_interface_already_available(&mut config)? == InterfaceResolution::SkipSetup {
        println!(
            "Keeping existing interface '{}' and skipping setup.",
            config.iface()
        );
        return Ok(());
    }

    execute_config(&config)?;
    println!("Set-up new interface '{}' successfully.", config.iface());

    Ok(())
}

pub fn run_setup_from_cli() -> Result<()> {
    let _ = run_setup_from_cli_and_return_config()?;
    Ok(())
}

pub fn run_setup_from_cli_and_return_config() -> Result<Option<CanConfig>> {
    prereqs::handle_missing_prerequisites()?;

    let mode = Select::new(
        "Select CAN connection type:",
        vec![CanMode::Native, CanMode::Slcan, CanMode::Virtual],
    )
    .prompt()
    .context("failed to read CAN mode")?;

    let mut config = match mode {
        CanMode::Native => CanConfig::Native(prompt_native()?),
        CanMode::Slcan => CanConfig::Slcan(prompt_slcan()?),
        CanMode::Virtual => CanConfig::Virtual(prompt_virtual()?),
    };

    if ensure_interface_name_is_available(&mut config)? == InterfaceResolution::SkipSetup {
        println!(
            "Keeping existing interface '{}' and skipping setup.",
            config.iface()
        );
        return Ok(Some(config));
    }

    print_plan(&config);

    let execute = Select::new(
        "What do you want to do?",
        vec!["Execute now", "Only print commands"],
    )
    .prompt()
    .context("failed to read execution mode")?;

    if execute == "Execute now" {
        execute_config(&config)?;
        println!("Done.");
    }

    Ok(Some(config))
}
