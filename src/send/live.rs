use anyhow::{Result, anyhow};
use inquire::Select;
use socketcan::{CanFrame, CanSocket, EmbeddedFrame, Socket, StandardId};
use std::thread;
use std::time::Duration;

use super::parse::SendFrame;

pub fn send_once(iface: &str, frame: &SendFrame) -> Result<()> {
    let socket = CanSocket::open(iface)?;

    let id = StandardId::new(frame.id as u16)
        .ok_or_else(|| anyhow!("invalid standard CAN ID {:03X}", frame.id))?;

    let can_frame =
        CanFrame::new(id, &frame.data).ok_or_else(|| anyhow!("failed to create CAN frame"))?;

    socket.write_frame(&can_frame)?;
    Ok(())
}

pub fn send_manual_repeat(iface: &str, frame: &SendFrame) -> Result<()> {
    loop {
        send_once(iface, frame)?;

        let action =
            Select::new("What do you want to do next?", vec!["Send again", "Done"]).prompt()?;

        match action {
            "Send again" => {}
            _ => return Ok(()),
        }
    }
}

pub fn send_cyclic(iface: &str, frame: &SendFrame, interval: Duration) -> Result<()> {
    loop {
        send_once(iface, frame)?;
        thread::sleep(interval);
    }
}
