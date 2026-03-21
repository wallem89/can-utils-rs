use anyhow::Result;
use owo_colors::OwoColorize;

mod live;
mod parse;
mod prompt;

pub use parse::SendFrame;

pub fn run_send_wizard() -> Result<()> {
    let iface = prompt::prompt_can_interface()?;
    let frame = prompt::prompt_frame()?;
    let mode = prompt::prompt_send_mode()?;

    println!();
    println!("{}", "══════════════════════════════════════".dimmed());
    println!("{} {}", "Sending on".bold(), iface.cyan().bold());
    println!("{} {}", "Frame:".bold(), frame.format_spaced().green());
    println!("{}", "══════════════════════════════════════".dimmed());
    println!();

    let interval = match mode {
        prompt::SendMode::Once => {
            live::send_once(&iface, &frame)?;
            println!("{}", "Frame sent.".green().bold());
            return Ok(());
        }
        prompt::SendMode::ManualRepeat => {
            live::send_manual_repeat(&iface, &frame)?;
            return Ok(());
        }
        prompt::SendMode::CyclicInterval => {
            let interval = prompt::prompt_interval()?;
            println!();
            println!("{}", "Starting cyclic CAN send".bold());
            println!("{} {}", "Interface:".bold(), iface.cyan().bold());
            println!("{} {}", "Frame:".bold(), frame.format_spaced().green());
            println!(
                "{} {} ms",
                "Interval:".bold(),
                interval.as_millis().to_string().green()
            );
            interval
        }
        prompt::SendMode::CyclicFrequency => {
            let interval = prompt::prompt_frequency()?;
            println!();
            println!("{}", "Starting cyclic CAN send".bold());
            println!("{} {}", "Interface:".bold(), iface.cyan().bold());
            println!("{} {}", "Frame:".bold(), frame.format_spaced().green());
            println!(
                "{} {} hz",
                "Frequency:".bold(),
                (1000 / interval.as_millis()).to_string().green()
            );
            interval
        }
    };
    println!("{}", "Press Ctrl+C to stop.".yellow());
    println!();

    live::send_cyclic(&iface, &frame, interval)?;

    Ok(())
}
