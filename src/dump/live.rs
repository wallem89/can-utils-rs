use anyhow::Result;
use socketcan::{CanFilter, CanSocket, EmbeddedFrame, Frame, Socket, SocketOptions};
use std::time::{SystemTime, UNIX_EPOCH};

use super::DumpFilter;
use super::format::format_frame;

pub fn dump_raw(iface: &str) -> Result<()> {
    let socket = CanSocket::open(iface)?;
    dump_socket(iface, socket)
}

pub fn dump_raw_filtered(iface: &str, filters: &[DumpFilter]) -> Result<()> {
    let socket = CanSocket::open(iface)?;
    let socket_filters = filters
        .iter()
        .map(|filter| CanFilter::new(filter.id, filter.mask))
        .collect::<Vec<_>>();

    socket.set_filters(&socket_filters)?;
    dump_socket(iface, socket)
}

fn dump_socket(iface: &str, socket: CanSocket) -> Result<()> {
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
