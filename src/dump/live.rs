use anyhow::Result;
use socketcan::{CanSocket, EmbeddedFrame, Frame, Socket};
use std::time::{SystemTime, UNIX_EPOCH};

use super::format::format_frame;

pub fn dump_raw(iface: &str) -> Result<()> {
    let socket = CanSocket::open(iface)?;

    loop {
        let frame = socket.read_frame()?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch");

        let ts = format!("({}.{:06})", now.as_secs(), now.subsec_micros());

        let id = frame.raw_id();
        let data = frame.data();

        let line = format_frame(&ts, iface, id, data);

        println!("{}", line);
    }
}
