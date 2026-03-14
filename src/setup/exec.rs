use anyhow::{Context, Result, bail};
use std::process::{Command, Stdio};

use crate::setup::models::{CanConfig, ExistingIfaceAction, InterfaceResolution};
use crate::setup::prompt::{prompt_existing_interface_action, prompt_new_interface_name};

pub fn interface_exists(iface: &str) -> bool {
    Command::new("ip")
        .args(["link", "show", iface])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

pub fn remove_existing_interface(config: &CanConfig) -> Result<()> {
    let iface = config.iface();

    let _ = run_sudo(&["ip", "link", "set", iface, "down"]);

    match config {
        CanConfig::Native(_) => {
            // Physical CAN interfaces are usually reconfigured after bringing them down.
        }
        CanConfig::Slcan(_) => {
            let _ = run_sudo(&["slcand", "-k", iface]);
        }
        CanConfig::Virtual(_) => {
            run_sudo(&["ip", "link", "delete", iface])?;
        }
    }

    Ok(())
}

pub fn check_interface_already_available(config: &mut CanConfig) -> Result<InterfaceResolution> {
    let iface = config.iface().to_string();

    if interface_exists(&iface) {
        execute_existing_set_up(config)?;
        Ok(InterfaceResolution::SkipSetup)
    } else {
        Ok(InterfaceResolution::Proceed)
    }
}

pub fn ensure_interface_name_is_available(config: &mut CanConfig) -> Result<InterfaceResolution> {
    loop {
        let iface = config.iface().to_string();

        if !interface_exists(&iface) {
            return Ok(InterfaceResolution::Proceed);
        }

        match prompt_existing_interface_action(&iface)? {
            ExistingIfaceAction::Replace => {
                remove_existing_interface(config)?;
                return Ok(InterfaceResolution::Proceed);
            }
            ExistingIfaceAction::Rename => {
                let new_iface = prompt_new_interface_name(&iface)?;
                config.set_iface(new_iface);
            }
            ExistingIfaceAction::Skip => {
                execute_existing_set_up(config)?;
                return Ok(InterfaceResolution::SkipSetup);
            }
            ExistingIfaceAction::Cancel => {
                bail!("operation cancelled by user");
            }
        }
    }
}

pub fn execute_existing_set_up(config: &CanConfig) -> Result<()> {
    let iface = match config {
        CanConfig::Native(cfg) => cfg.iface.as_str(),
        CanConfig::Slcan(cfg) => cfg.iface.as_str(),
        CanConfig::Virtual(cfg) => cfg.iface.as_str(),
    };
    // For any type interface just make sure to bring it up, as it should already be configured correctly if it exists
    run_sudo(&["ip", "link", "set", "up", iface])?;
    Ok(())
}

pub fn execute_config(config: &CanConfig) -> Result<()> {
    match config {
        CanConfig::Native(cfg) => {
            let bitrate = cfg.bitrate.bitrate().to_string();
            run_sudo(&[
                "ip",
                "link",
                "set",
                cfg.iface.as_str(),
                "up",
                "type",
                "can",
                "bitrate",
                bitrate.as_str(),
            ])?;
        }
        CanConfig::Slcan(cfg) => {
            let speed_arg = format!("-{}", cfg.speed.flag);
            let baud_arg = cfg.uart_baud.to_string();

            run_sudo(&[
                "slcand",
                "-c",
                "-o",
                "-f",
                speed_arg.as_str(),
                "-t",
                "hw",
                "-S",
                baud_arg.as_str(),
                cfg.tty.as_str(),
                cfg.iface.as_str(),
            ])?;

            run_sudo(&["ip", "link", "set", "up", cfg.iface.as_str()])?;
        }
        CanConfig::Virtual(cfg) => {
            run_sudo(&[
                "ip",
                "link",
                "add",
                "dev",
                cfg.iface.as_str(),
                "type",
                "vcan",
            ])?;
            run_sudo(&["ip", "link", "set", "up", cfg.iface.as_str()])?;
            run(&["ip", "link", "show", cfg.iface.as_str()])?;
        }
    }

    Ok(())
}

pub(crate) fn run_sudo(args: &[&str]) -> Result<()> {
    let status = Command::new("sudo")
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to start sudo {:?}", args))?;

    if !status.success() {
        bail!("command failed: sudo {:?}", args);
    }

    Ok(())
}

pub fn run(args: &[&str]) -> Result<()> {
    let (program, rest) = args.split_first().context("empty command provided")?;

    let status = Command::new(program)
        .args(rest)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to start {:?}", args))?;

    if !status.success() {
        bail!("command failed: {:?}", args);
    }

    Ok(())
}
