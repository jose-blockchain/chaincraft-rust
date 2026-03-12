//! ChainCraft CLI application
//!
//! Matches Python chaincraft-cli UX: start node, choose port, connect to seed peer, debug/memory.

use chaincraft::{ChaincraftNode, Result};
use clap::{Parser, Subcommand};
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{info, Level};

#[derive(Parser)]
#[command(name = "chaincraft-cli")]
#[command(about = "A high-performance blockchain education and prototyping platform")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Port to listen on
    #[arg(short = 'p', long)]
    port: Option<u16>,

    /// Use a random (ephemeral) port
    #[arg(short = 'r', long)]
    random_port: bool,

    /// Maximum number of peers
    #[arg(short = 'n', long, default_value = "10")]
    max_peers: usize,

    /// Enable debug logging
    #[arg(short = 'd', long)]
    debug: bool,

    /// Use memory storage instead of persistent storage
    #[arg(short = 'm', long)]
    memory: bool,

    /// Seed peer to connect to (host:port)
    #[arg(short = 's', long)]
    seed_peer: Option<String>,

    /// Set verbosity level (0-4)
    #[arg(short = 'v', long, default_value_t = 2)]
    verbosity: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a ChainCraft node (default)
    Start,
    /// Generate a new keypair
    Keygen,
    /// Show version information
    Version,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let level = if cli.debug {
        Level::DEBUG
    } else {
        match cli.verbosity {
            0 => Level::ERROR,
            1 => Level::WARN,
            2 => Level::INFO,
            3 => Level::DEBUG,
            _ => Level::TRACE,
        }
    };
    tracing_subscriber::fmt().with_max_level(level).init();

    match &cli.command {
        Some(Commands::Start) | None => {
            let port = if cli.random_port {
                0u16
            } else {
                cli.port.unwrap_or(21000)
            };

            let mut node = ChaincraftNode::builder()
                .port(port)
                .max_peers(cli.max_peers)
                .with_persistent_storage(!cli.memory)
                .build()?;

            node.start().await?;

            if let Some(addr) = &cli.seed_peer {
                if let Err(e) = node.connect_to_peer(addr).await {
                    tracing::warn!("Failed to connect to seed peer {}: {:?}", addr, e);
                } else {
                    info!("Connected to seed peer {}", addr);
                }
            }

            println!("Node started on {}:{}", node.host(), node.port());
            println!("Enter a message to broadcast (Ctrl+C to quit):");
            println!("Usage: chaincraft-cli [-d] [-p PORT] [-r] [-m] [-s HOST:PORT]");

            let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(32);
            tokio::spawn(async move {
                let mut lines = BufReader::new(tokio::io::stdin()).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = tx.send(line).await;
                }
            });

            loop {
                tokio::select! {
                    Some(line) = rx.recv() => {
                        let line = line.trim();
                        if !line.is_empty() {
                            let data = serde_json::json!(line);
                            if let Err(e) = node.create_shared_message_with_data(data).await {
                                tracing::warn!("Failed to broadcast: {:?}", e);
                            }
                        }
                    }
                    _ = tokio::signal::ctrl_c() => {
                        info!("Shutting down...");
                        break;
                    }
                }
            }
            node.stop().await?;
        },
        Some(Commands::Keygen) => {
            use chaincraft::crypto::{utils, KeyType};

            let (private_key, public_key) = utils::generate_keypair(KeyType::Secp256k1)?;

            println!("Generated new keypair:");
            println!("Private key: {}", private_key.to_hex());
            println!("Public key: {}", public_key.to_hex());

            use chaincraft::crypto::address::Address;
            let address = Address::from_public_key(&public_key);
            println!("Address: {address}");
        },
        Some(Commands::Version) => {
            println!("ChainCraft Rust v{}", chaincraft::VERSION);
        },
    }

    Ok(())
}
