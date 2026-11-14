use std::path::PathBuf;

use clap::Parser;
use modbusio::{server, storage};

mod cli;

#[derive(Parser)]
#[command(
    version = concat!(env!("CARGO_PKG_VERSION"), " ", env!("MODBUSIO_COMMIT")),
    about = "Modbus browser. Without a command it opens in the browser."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<cli::Command>,
    /// Port of the local web server
    #[arg(long, default_value_t = server::DEFAULT_PORT)]
    web_port: u16,
    /// Do not open the browser
    #[arg(long)]
    no_browser: bool,
    /// Folder for the write log, profiles and sites (default: the user's local application data)
    #[arg(long, value_name = "DIR", global = true)]
    data_dir: Option<PathBuf>,
    /// Offer the loopback interface as an adapter, for the browser tests' simulated devices
    #[arg(long, hide = true)]
    test_loopback: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let data_dir = cli.data_dir.or_else(storage::default_dir);
    let Some(command) = cli.command else {
        let served =
            server::serve(cli.web_port, !cli.no_browser, data_dir, cli.test_loopback).await;
        if let Err(error) = &served {
            eprintln!("Error: {error:#}");
            wait_for_enter();
        }
        return served;
    };
    cli::run(command, data_dir).await
}

/// A double-clicked program on Windows closes its window at once; keep the error readable.
fn wait_for_enter() {
    if cfg!(windows) {
        eprintln!("Press Enter to close.");
        let _ = std::io::stdin().read_line(&mut String::new());
    }
}
