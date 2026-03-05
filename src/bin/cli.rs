//! ChainCraft CLI application

use chaincraft_rust::{ChaincraftNode, Result};
use clap::{Parser, Subcommand};
use tracing::{info, Level};

#[derive(Parser)]
#[command(name = "chaincraft-cli")]
#[command(about = "A high-performance blockchain education and prototyping platform")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Port to listen on
    #[arg(short, long, default_value = "21000")]
    port: u16,

    /// Maximum number of peers
    #[arg(short = 'n', long, default_value = "10")]
    max_peers: usize,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Use memory storage instead of persistent storage
    #[arg(short = 'm', long)]
    memory: bool,

    /// Set verbosity level (0-4)
    #[arg(short = 'v', long, default_value_t = 2)]
    verbosity: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a ChainCraft node
    Start,
    /// Generate a new keypair
    Keygen,
    /// Show version information
    Version,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let level = match cli.verbosity {
        0 => Level::ERROR,
        1 => Level::WARN,
        2 => Level::INFO,
        3 => Level::DEBUG,
        _ => Level::TRACE,
    };

    tracing_subscriber::fmt().with_max_level(level).init();

    match &cli.command {
        Some(Commands::Start) | None => {
            info!("Starting ChainCraft node on port {}", cli.port);

            let mut node = ChaincraftNode::builder()
                .port(cli.port)
                .max_peers(cli.max_peers)
                .with_persistent_storage(!cli.memory)
                .build()?;

            info!("Node {} started on port {}", node.id(), node.port());

            node.start().await?;

            // Keep the node running
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for ctrl-c");

            info!("Shutting down node...");
            node.stop().await?;
        },
        Some(Commands::Keygen) => {
            use chaincraft_rust::crypto::{utils, KeyType};

            let (private_key, public_key) = utils::generate_keypair(KeyType::Secp256k1)?;

            println!("Generated new keypair:");
            println!("Private key: {}", private_key.to_hex());
            println!("Public key: {}", public_key.to_hex());

            use chaincraft_rust::crypto::address::Address;
            let address = Address::from_public_key(&public_key);
            println!("Address: {}", address);
        },
        Some(Commands::Version) => {
            println!("ChainCraft Rust v{}", chaincraft_rust::VERSION);
        },
    }

    Ok(())
}
