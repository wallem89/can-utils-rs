use owo_colors::OwoColorize;

pub fn format_frame(timestamp: &str, iface: &str, id: u32, data: &[u8]) -> String {
    let mut payload = String::new();

    for (i, byte) in data.iter().enumerate() {
        let colored: String = match i % 8 {
            0 => format!("{:02X}", byte).bright_red().to_string(),
            1 => format!("{:02X}", byte).bright_yellow().to_string(),
            2 => format!("{:02X}", byte).bright_green().to_string(),
            3 => format!("{:02X}", byte).bright_magenta().to_string(),
            4 => format!("{:02X}", byte).bright_cyan().to_string(),
            5 => format!("{:02X}", byte).bright_white().to_string(),
            6 => format!("{:02X}", byte).bright_blue().to_string(),
            _ => format!("{:02X}", byte).bright_green().to_string(),
        };
        payload.push_str(&colored);
        payload.push(' ');
    }

    let payload = payload.trim_end();

    format!(
        "{} {} {}#{}",
        timestamp.dimmed(),
        iface.cyan(),
        format!("{:03X}", id).blue().bold(),
        payload
    )
}
