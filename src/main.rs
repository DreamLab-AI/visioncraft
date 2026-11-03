//! VisionCraft's explicit command-line entry point and cancellation bridge.
use clap::Parser;
use std::process::ExitCode;
use std::sync::{Arc, atomic::AtomicBool};
use visioncraft::cli::Cli;

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let failed = error.use_stderr();
            let _ = error.print();
            let legacy = std::env::args().nth(1).as_deref() == Some("deepsec");
            return ExitCode::from(if !failed {
                0
            } else if legacy {
                78
            } else {
                2
            });
        }
    };
    let cancelled = Arc::new(AtomicBool::new(false));
    #[cfg(unix)]
    for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
        if signal_hook::flag::register(signal, cancelled.clone()).is_err() {
            eprintln!("visioncraft: could not install cancellation handler");
            return ExitCode::from(70);
        }
    }
    match visioncraft::cli::execute(cli, &cancelled) {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(70)),
        Err(error) => {
            eprintln!("visioncraft: {}", error.message);
            ExitCode::from(u8::try_from(error.code).unwrap_or(70))
        }
    }
}
