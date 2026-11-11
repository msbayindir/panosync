//! panosync: entry point. Parses the command line and runs the chosen command.

mod cli;
mod clipboard;
mod config;

use anyhow::{Result, bail};
use clap::Parser;
use cli::{Cli, Command};
use config::{Config, ConfigDir};
use panosync::{crypto, net, pause, unix_now};
use tokio::sync::mpsc;

fn main() -> Result<()> {
    init_logging();
    let dir = ConfigDir::locate()?;

    match Cli::parse().command.unwrap_or(Command::Start) {
        Command::Setup { server, key } => setup(&dir, server, key),
        Command::Start => start(&dir),
        Command::Key => {
            println!("{}", Config::load(&dir.config_file())?.key);
            Ok(())
        }
        Command::Config => {
            println!("{}", dir.config_file().display());
            Ok(())
        }
        Command::Pause { minutes } => pause_sync(&dir, minutes),
    }
}

fn init_logging() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stdout()))
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
}

fn setup(dir: &ConfigDir, server: String, key: Option<String>) -> Result<()> {
    if !(server.starts_with("wss://") || server.starts_with("ws://")) {
        bail!("the server address must start with wss://");
    }
    if server.starts_with("ws://") {
        eprintln!(
            "WARNING: ws:// does not use TLS. Content is still end-to-end encrypted, but use wss:// for real use."
        );
    }

    let is_new_key = key.is_none();
    let key = key.unwrap_or_else(crypto::generate_master_key);
    crypto::Keys::from_master_key(&key)?; // reject a malformed key before saving it

    let path = dir.config_file();
    Config {
        server: server.clone(),
        key: key.clone(),
    }
    .save(&path)?;
    println!("Config saved: {}", path.display());

    if is_new_key {
        println!("\nA new key was generated. On your other device, run:\n");
        println!("  panosync setup --server {server} --key {key}\n");
        println!("Anyone who knows this key can read your clipboard. Never share it;");
        println!("only move it to your own devices over a secure channel.");
    }
    println!("\nTo start: panosync");
    Ok(())
}

fn start(dir: &ConfigDir) -> Result<()> {
    let config = Config::load(&dir.config_file())?;
    let keys = crypto::Keys::from_master_key(&config.key)?;
    let pause_file = dir.pause_file();
    // TLS for wss:// connections.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let (outgoing_tx, outgoing_rx) = mpsc::channel::<String>(16);
    let (incoming_tx, incoming_rx) = mpsc::channel::<String>(16);

    std::thread::spawn(move || {
        if let Err(e) = clipboard::run(outgoing_tx, incoming_rx, pause_file) {
            tracing::error!("clipboard error: {e:#}");
            std::process::exit(1);
        }
    });

    tracing::info!("panosync is running, press Ctrl+C to stop");
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(net::run(config.server, keys, outgoing_rx, incoming_tx))
}

fn pause_sync(dir: &ConfigDir, minutes: u64) -> Result<()> {
    match pause::pause(&dir.pause_file(), minutes, unix_now())? {
        Some(_) => println!(
            "Paused for {minutes} minute(s). The running panosync will not send clipboard changes.\nResume early with: panosync pause 0"
        ),
        None => println!("Resumed. Clipboard changes will be sent again."),
    }
    Ok(())
}
