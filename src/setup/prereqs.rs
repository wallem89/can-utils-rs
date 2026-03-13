use anyhow::{Result, bail};
use inquire::Select;
use std::env;
use std::fmt;
use std::path::Path;

use crate::setup::exec::run_sudo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prerequisite {
    Ip,
    Slcand,
    Modprobe,
}

impl Prerequisite {
    pub fn binary_name(&self) -> &'static str {
        match self {
            Prerequisite::Ip => "ip",
            Prerequisite::Slcand => "slcand",
            Prerequisite::Modprobe => "modprobe",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Prerequisite::Ip => "iproute2 / ip",
            Prerequisite::Slcand => "can-utils / slcand",
            Prerequisite::Modprobe => "kmod / modprobe",
        }
    }
}

impl fmt::Display for Prerequisite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupAction {
    InstallPrerequisites,
    ContinueAnyway,
    Exit,
}

impl fmt::Display for StartupAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StartupAction::InstallPrerequisites => write!(f, "Install prerequisites"),
            StartupAction::ContinueAnyway => write!(f, "Continue anyway"),
            StartupAction::Exit => write!(f, "Exit"),
        }
    }
}

pub fn command_exists(cmd: &str) -> bool {
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&paths).any(|dir| Path::new(&dir).join(cmd).exists())
}

pub fn has_apt() -> bool {
    command_exists("apt")
}

pub fn missing_prerequisites() -> Vec<Prerequisite> {
    let all = [
        Prerequisite::Ip,
        Prerequisite::Slcand,
        Prerequisite::Modprobe,
    ];

    all.into_iter()
        .filter(|p| !command_exists(p.binary_name()))
        .collect()
}

pub fn print_missing_prerequisites(missing: &[Prerequisite]) {
    if missing.is_empty() {
        return;
    }

    eprintln!("Missing prerequisites:");
    for item in missing {
        eprintln!("  - {}", item.description());
    }
    eprintln!();
}

pub fn install_missing_prerequisites(missing: &[Prerequisite]) -> Result<()> {
    if missing.is_empty() {
        return Ok(());
    }

    if !has_apt() {
        bail!("automatic installation is currently only supported on apt-based systems");
    }

    let mut packages = Vec::new();

    if missing.iter().any(|p| matches!(p, Prerequisite::Ip)) {
        packages.push("iproute2");
    }
    if missing.iter().any(|p| matches!(p, Prerequisite::Slcand)) {
        packages.push("can-utils");
    }
    if missing.iter().any(|p| matches!(p, Prerequisite::Modprobe)) {
        packages.push("kmod");
    }

    packages.sort_unstable();
    packages.dedup();

    println!("The following packages will be installed:");
    for pkg in &packages {
        println!("  - {pkg}");
    }
    println!();

    let confirm = Select::new("Proceed with installation?", vec!["Yes", "No"]).prompt()?;

    if confirm != "Yes" {
        bail!("installation cancelled by user");
    }

    run_sudo(&["apt", "update"])?;

    let mut args: Vec<&str> = vec!["apt", "install", "-y"];
    args.extend(packages.iter().copied());

    run_sudo(&args)?;

    let still_missing = missing_prerequisites();
    if !still_missing.is_empty() {
        print_missing_prerequisites(&still_missing);
        bail!("some prerequisites are still missing after installation");
    }

    Ok(())
}

pub fn handle_missing_prerequisites() -> Result<()> {
    let missing = missing_prerequisites();

    if missing.is_empty() {
        return Ok(());
    }

    print_missing_prerequisites(&missing);

    let mut actions = Vec::new();
    if has_apt() {
        actions.push(StartupAction::InstallPrerequisites);
    }
    actions.push(StartupAction::ContinueAnyway);
    actions.push(StartupAction::Exit);

    let action = Select::new(
        "Some required tools are missing. What do you want to do?",
        actions,
    )
    .prompt()?;

    match action {
        StartupAction::InstallPrerequisites => install_missing_prerequisites(&missing)?,
        StartupAction::ContinueAnyway => {}
        StartupAction::Exit => bail!("exiting because prerequisites are missing"),
    }

    Ok(())
}
