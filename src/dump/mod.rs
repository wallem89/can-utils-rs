use anyhow::{Context, Result, anyhow, bail};
use inquire::{Select, Text};
use owo_colors::OwoColorize;
use std::fmt;
use std::process::Command;

mod format;
mod live;

#[derive(Debug, Clone, Copy)]
pub(crate) struct DumpFilter {
    id: u32,
    mask: u32,
}

impl DumpFilter {
    fn format_candump(&self) -> String {
        format!("0x{:X}:0x{:X}", self.id, self.mask)
    }
}

#[derive(Debug, Clone, Copy)]
enum DumpMode {
    AllFrames,
    FilterById,
}

impl fmt::Display for DumpMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DumpMode::AllFrames => write!(f, "Dump all frames"),
            DumpMode::FilterById => write!(f, "Filter by one or more CAN IDs"),
        }
    }
}

pub fn run_dump(iface: &str) -> anyhow::Result<()> {
    let filters = prompt_dump_filters()?;

    run_dump_with_filters(iface, filters)
}

fn run_dump_with_filters(iface: &str, filters: Vec<DumpFilter>) -> anyhow::Result<()> {
    let target = format_dump_target(iface, &filters);

    println!("{} {}", "Pretty CAN Dump on".bold(), iface.cyan().bold());
    if !filters.is_empty() {
        println!("{} {}", "Applying filters:".bold(), target.cyan());
    }

    println!("{}", "Press Ctrl+C to stop.".yellow());

    if filters.is_empty() {
        live::dump_raw(iface)?;
    } else {
        live::dump_raw_filtered(iface, &filters)?;
    }

    Ok(())
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

    let filters = prompt_dump_filters()?;
    run_dump_with_filters(&iface, filters)?;

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

fn prompt_dump_filters() -> Result<Vec<DumpFilter>> {
    let mode = Select::new(
        "How do you want to dump frames?",
        vec![DumpMode::AllFrames, DumpMode::FilterById],
    )
    .prompt()
    .context("failed to read dump mode")?;

    match mode {
        DumpMode::AllFrames => Ok(Vec::new()),
        DumpMode::FilterById => {
            let input = Text::new("Enter CAN ID filters:")
                .with_help_message("Examples: 123,456 or 0x123:0x7FF,0x456:0x7FF")
                .prompt()
                .context("failed to read CAN ID filters")?;

            parse_dump_filters(&input)
        }
    }
}

fn parse_dump_filters(input: &str) -> Result<Vec<DumpFilter>> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        bail!("at least one CAN ID filter is required");
    }

    trimmed
        .split(',')
        .map(parse_dump_filter)
        .collect::<Result<Vec<_>>>()
}

fn parse_dump_filter(input: &str) -> Result<DumpFilter> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        bail!("empty CAN ID filter");
    }

    let (id, mask) = if let Some((id, mask)) = trimmed.split_once(':') {
        (
            parse_hex_u32(id, "CAN ID")?,
            parse_hex_u32(mask, "filter mask")?,
        )
    } else {
        let id = parse_hex_u32(trimmed, "CAN ID")?;
        let mask = if id <= 0x7FF { 0x7FF } else { 0x1FFF_FFFF };
        (id, mask)
    };

    validate_can_id(id)?;
    validate_can_mask(mask)?;

    Ok(DumpFilter { id, mask })
}

fn parse_hex_u32(input: &str, label: &str) -> Result<u32> {
    let trimmed = input.trim();
    let hex = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);

    if hex.is_empty() {
        bail!("{label} cannot be empty");
    }

    u32::from_str_radix(hex, 16).map_err(|_| anyhow!("{label} must be valid hexadecimal"))
}

fn validate_can_id(id: u32) -> Result<()> {
    if id > 0x1FFF_FFFF {
        bail!("CAN ID must be <= 0x1FFFFFFF");
    }

    Ok(())
}

fn validate_can_mask(mask: u32) -> Result<()> {
    if mask > 0x1FFF_FFFF {
        bail!("filter mask must be <= 0x1FFFFFFF");
    }

    Ok(())
}

fn format_dump_target(iface: &str, filters: &[DumpFilter]) -> String {
    if filters.is_empty() {
        return iface.to_string();
    }

    let filters = filters
        .iter()
        .map(DumpFilter::format_candump)
        .collect::<Vec<_>>()
        .join(",");

    format!("{iface},{filters}")
}

#[cfg(test)]
mod tests {
    use super::{format_dump_target, parse_dump_filters};

    #[test]
    fn parses_comma_separated_ids_with_standard_default_mask() {
        let filters = parse_dump_filters("123, 0x456").unwrap();

        assert_eq!(filters.len(), 2);
        assert_eq!(filters[0].id, 0x123);
        assert_eq!(filters[0].mask, 0x7FF);
        assert_eq!(filters[1].id, 0x456);
        assert_eq!(filters[1].mask, 0x7FF);
    }

    #[test]
    fn parses_candump_style_filters() {
        let filters = parse_dump_filters("0x123:0x7FF,0x456:0x7FF").unwrap();

        assert_eq!(filters.len(), 2);
        assert_eq!(filters[0].id, 0x123);
        assert_eq!(filters[0].mask, 0x7FF);
        assert_eq!(filters[1].id, 0x456);
        assert_eq!(filters[1].mask, 0x7FF);
    }

    #[test]
    fn formats_candump_style_target() {
        let filters = parse_dump_filters("123,456").unwrap();

        assert_eq!(
            format_dump_target("can3", &filters),
            "can3,0x123:0x7FF,0x456:0x7FF"
        );
    }
}
