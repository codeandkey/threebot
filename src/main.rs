mod audio;
mod config;
mod error;
mod protos;
mod session;
mod verifier;
mod util;
mod commands;
mod sounds;
mod alias;
mod user_settings;
mod database;

#[macro_use]
extern crate log;

use clap::Parser;
use config::BotConfig;
use std::error::Error;

#[derive(Parser)]
#[command(name = "bigbot", version = "0.1.0", author = "Justin Stanley")]
struct Cli {
    #[arg(short, long, help = "Enable verbose output (overrides config)")]
    verbose: bool,

    #[arg(short, long, help = "Data directory path (overrides config)")]
    data_dir: Option<String>,
    
    #[arg(short, long, help = "Configuration file path (default: ~/.bigbot/config.yml)")]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Load configuration (will auto-create from example if needed)
    let config_path = cli.config
        .map(|p| std::path::PathBuf::from(p))
        .unwrap_or_else(|| BotConfig::get_config_path());
    
    let mut config = BotConfig::load_or_create(&config_path)?;
    
    // Apply CLI overrides
    config.apply_cli_overrides(
        if cli.verbose { Some(true) } else { None },
        cli.data_dir
    );

    // Set up logging based on configuration
    if config.bot.verbose {
        pretty_env_logger::formatted_builder()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        pretty_env_logger::formatted_builder()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    info!("Starting Big Bot v{}", env!("CARGO_PKG_VERSION"));
    info!("Using configuration from: {}", config_path.display());

    let data_dir = config.get_data_dir();

    // Ensure the data directory exists
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data directory {}: {}", data_dir.display(), e))?;
        info!("Created data directory at {}", data_dir.display());
    }

    // Get certificate and key paths from configuration
    let cert_path = config.get_cert_path();
    let key_path = config.get_key_path();

    let has_cert = cert_path.exists();
    let has_key = key_path.exists();

    // Generate self-signed chain if none exists
    if !has_cert || !has_key {
        if has_cert || has_key {
            return Err("Both certificate and key must be present or absent".into());
        }

        info!(
            "No certificate found at {}, generating self-signed certificate...",
            cert_path.display()
        );
        std::process::Command::new("openssl")
            .args(&[
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-keyout",
                key_path.to_str().unwrap(),
                "-out",
                cert_path.to_str().unwrap(),
                "-days",
                "365",
                "-nodes",
                "-subj",
                &format!("/CN={}", config.bot.username),
            ])
            .status()
            .expect("Failed to generate self-signed certificate");
    } else {
        info!("Using existing certificate at {}", cert_path.display());
    }

    let session = session::Session::new(session::ConnectionOptions {
        host: config.server.host,
        port: config.server.port,
        username: config.bot.username,
        cert: cert_path.to_string_lossy().to_string(),
        key: key_path.to_string_lossy().to_string(),
        password: config.bot.password,
        timeout: Some(config.server.timeout_seconds),
        data_dir: Some(data_dir.to_string_lossy().to_string()),
        behavior_settings: config.behavior,
        audio_effects: config.audio_effects,
    })
    .await?;

    session.start_main_loop().await?;

    Ok(())
}
