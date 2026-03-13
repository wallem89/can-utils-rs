use crate::setup::models::AppConfig;

pub fn plan_lines(config: &AppConfig) -> Vec<String> {
    match config {
        AppConfig::Native(cfg) => vec![format!(
            "sudo ip link set {} up type can bitrate {}",
            cfg.iface, cfg.bitrate.bitrate
        )],
        AppConfig::Slcan(cfg) => vec![
            format!(
                "sudo slcand -c -o -f -{} -t hw -S {} {} {}",
                cfg.speed.flag, cfg.uart_baud, cfg.tty, cfg.iface
            ),
            format!("sudo ip link set up {}", cfg.iface),
        ],
        AppConfig::Virtual(cfg) => vec![
            format!("sudo ip link add dev {} type vcan", cfg.iface),
            format!("sudo ip link set up {}", cfg.iface),
            format!("ip link show {}", cfg.iface),
        ],
    }
}

pub fn print_plan(config: &AppConfig) {
    println!("\nPlanned commands:\n");

    for line in plan_lines(config) {
        println!("{line}");
    }

    println!();
}
