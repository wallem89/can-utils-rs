use anyhow::{Context, Result};
use inquire::Select;

pub mod exec;
pub mod models;
pub mod plan;
pub mod prereqs;
pub mod prompt;

use exec::{ensure_interface_name_is_available, execute_config};
use models::{AppConfig, CanMode, InterfaceResolution};
use plan::print_plan;
use prompt::{prompt_native, prompt_slcan, prompt_virtual};

pub fn run_setup() -> Result<()> {
    let _ = run_setup_and_return_config()?;
    Ok(())
}

pub fn run_setup_and_return_config() -> Result<Option<AppConfig>> {
    prereqs::handle_missing_prerequisites()?;

    let mode = Select::new(
        "Select CAN connection type:",
        vec![CanMode::Native, CanMode::Slcan, CanMode::Virtual],
    )
    .prompt()
    .context("failed to read CAN mode")?;

    let mut config = match mode {
        CanMode::Native => AppConfig::Native(prompt_native()?),
        CanMode::Slcan => AppConfig::Slcan(prompt_slcan()?),
        CanMode::Virtual => AppConfig::Virtual(prompt_virtual()?),
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
