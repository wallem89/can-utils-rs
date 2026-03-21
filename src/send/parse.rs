use anyhow::{Result, anyhow, bail};

#[derive(Debug, Clone)]
pub struct SendFrame {
    pub id: u32,
    pub data: Vec<u8>,
}

impl SendFrame {
    pub fn format_compact(&self) -> String {
        let payload = self
            .data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<String>();

        format!("{:03X}#{}", self.id, payload)
    }

    pub fn format_spaced(&self) -> String {
        if self.data.is_empty() {
            return format!("{:03X}#", self.id);
        }

        let payload = self
            .data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        format!("{:03X}#{}", self.id, payload)
    }
}

pub fn parse_can_id(input: &str) -> Result<u32> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        bail!("CAN ID cannot be empty");
    }

    let id = u32::from_str_radix(trimmed, 16)
        .map_err(|_| anyhow!("CAN ID must be valid hexadecimal"))?;

    if id > 0x7FF {
        bail!("CAN ID must be <= 7FF for standard CAN");
    }

    Ok(id)
}

pub fn parse_can_data(input: &str) -> Result<Vec<u8>> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let parts: Vec<&str> = trimmed.split_whitespace().collect();

    if parts.len() > 8 {
        bail!("CAN data may contain at most 8 bytes");
    }

    let mut data = Vec::with_capacity(parts.len());

    for part in parts {
        if part.len() != 2 {
            bail!("Each byte must be exactly 2 hexadecimal characters");
        }

        let byte =
            u8::from_str_radix(part, 16).map_err(|_| anyhow!("Invalid hex byte '{}'", part))?;

        data.push(byte);
    }

    Ok(data)
}
