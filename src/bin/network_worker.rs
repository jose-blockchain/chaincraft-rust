use chaincraft_rust::{
    network::PeerId,
    storage::MemoryStorage,
    ChaincraftNode,
};
use serde_json::json;
use std::{env, fs, path::PathBuf, sync::Arc, time::Duration};

/// Simple multi-process worker binary used only for end-to-end network tests.
///
/// Arguments (very simple parsing, no clap to keep dependencies minimal):
/// --port <u16>               : UDP port to bind
/// --peer <host:port> [...]   : one or more peers to connect to
/// --message <string>         : if present, create and broadcast this shared message
/// --output <path>            : file to write final db_size and a sample message (if any)
#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    let mut port: u16 = 0;
    let mut peers: Vec<String> = Vec::new();
    let mut message: Option<String> = None;
    let mut output: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" if i + 1 < args.len() => {
                port = args[i + 1].parse().expect("invalid port");
                i += 2;
            }
            "--peer" if i + 1 < args.len() => {
                peers.push(args[i + 1].clone());
                i += 2;
            }
            "--message" if i + 1 < args.len() => {
                message = Some(args[i + 1].clone());
                i += 2;
            }
            "--output" if i + 1 < args.len() => {
                output = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }

    // Create node
    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);
    if port != 0 {
        node.set_port(port);
    }

    // Start node
    node.start().await.expect("failed to start node");

    // Connect to peers
    for addr in &peers {
        let _ = node.connect_to_peer(addr).await;
    }

    // Give some time for connections to establish
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Optionally send a message
    if let Some(msg) = &message {
        let data = json!({ "type": "e2e_test", "payload": msg });
        let _ = node.create_shared_message_with_data(data).await;
    }

    // Let gossip / UDP propagation run for a bit
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Capture state before shutdown
    let db_size = node.db_size();

    // Try to read one stored message if any
    let sample = if db_size > 0 {
        // We don't have direct iteration over keys here; just report size.
        Some(json!({ "db_size": db_size }))
    } else {
        None
    };

    if let Some(path) = output {
        let content = json!({
            "port": node.port(),
            "db_size": db_size,
            "sample": sample,
        });
        fs::write(path, content.to_string()).expect("failed to write output file");
    }

    node.close().await.expect("failed to close node");
}