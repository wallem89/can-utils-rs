use crate::setup::models::CanConfig;

pub fn plan_lines(config: &CanConfig) -> Vec<String> {
    match config {
        CanConfig::Native(cfg) => vec![format!(
            "sudo ip link set {} up type can bitrate {}",
            cfg.iface,
            cfg.bitrate.bitrate()
        )],
        CanConfig::Slcan(cfg) => vec![
            format!(
                "sudo slcand -c -o -f -{} -t hw -S {} {} {}",
                cfg.speed.flag, cfg.uart_baud, cfg.tty, cfg.iface
            ),
            format!("sudo ip link set up {}", cfg.iface),
        ],
        CanConfig::Virtual(cfg) => vec![
            format!("sudo ip link add dev {} type vcan", cfg.iface),
            format!("sudo ip link set up {}", cfg.iface),
            format!("ip link show {}", cfg.iface),
        ],
    }
}

pub fn print_plan(config: &CanConfig) {
    println!("\nPlanned commands:\n");

    for line in plan_lines(config) {
        println!("{line}");
    }

    println!();
}
