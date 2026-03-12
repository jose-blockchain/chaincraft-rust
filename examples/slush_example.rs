//! Slush Consensus Example
//!
//! Demonstrates the Slush metastable consensus protocol from the Avalanche paper.
//! 10 ChaincraftNode instances reach agreement on a binary color (Red/Blue)
//! by broadcasting votes and adopting the majority each round.
//!
//! Run with: `cargo run --example slush_example`

use chaincraft::{
    clear_local_registry,
    error::Result,
    examples::slush::{create_vote_message, Color, SlushObject},
    network::PeerId,
    shared_object::ApplicationObject,
    storage::MemoryStorage,
    ChaincraftNode,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

async fn create_slush_node(
    node_id: &str,
    port: u16,
    k: usize,
    alpha: f64,
    m: u32,
) -> Result<ChaincraftNode> {
    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);
    node.set_port(port);
    let slush: Box<dyn ApplicationObject> =
        Box::new(SlushObject::new(node_id.to_string(), k, alpha, m));
    node.add_shared_object(slush).await?;
    node.start().await?;
    Ok(node)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    clear_local_registry();

    let num_nodes: usize = 10;
    let k: usize = 4;
    let alpha: f64 = 0.5;
    let m: u32 = 8;
    let base_port: u16 = 9400;
    let initial_color = Color::Red;

    println!("Slush Consensus Example (Avalanche paper)");
    println!("==========================================");
    println!("Nodes: {num_nodes}, k: {k}, alpha: {alpha}, rounds: {m}");
    println!("Proposer (node-0) initial color: {initial_color}\n");

    let mut nodes: Vec<ChaincraftNode> = Vec::new();
    for i in 0..num_nodes {
        let node_id = format!("node-{i}");
        let node = create_slush_node(&node_id, base_port + i as u16, k, alpha, m).await?;
        nodes.push(node);
    }

    for i in 0..num_nodes {
        for j in 0..num_nodes {
            if i != j {
                let addr = format!("{}:{}", nodes[j].host(), nodes[j].port());
                nodes[i].connect_to_peer(&addr).await?;
            }
        }
    }
    sleep(Duration::from_millis(500)).await;

    // Set proposer color
    {
        let objs = nodes[0].shared_objects().await;
        if let Some(obj) = objs.first() {
            if let Some(_slush) = obj.as_any().downcast_ref::<SlushObject>() {
                // We need mutable access via the registry
            }
        }
    }
    // Proposer broadcasts initial vote for round 0
    {
        let vote = create_vote_message("node-0", 0, initial_color);
        nodes[0].create_shared_message_with_data(vote).await?;
        println!("[node-0] Proposed {initial_color}");
    }
    sleep(Duration::from_millis(500)).await;

    // Run m rounds
    for round in 1..=m {
        // Each node broadcasts its current color
        for (i, node) in nodes.iter_mut().enumerate() {
            let node_id = format!("node-{i}");
            let color = {
                let objs = node.shared_objects().await;
                objs.first()
                    .and_then(|o| o.as_any().downcast_ref::<SlushObject>())
                    .and_then(|s| s.color)
                    .unwrap_or(initial_color)
            };
            let vote = create_vote_message(&node_id, round, color);
            node.create_shared_message_with_data(vote).await?;
        }

        sleep(Duration::from_millis(300)).await;

        // Each node processes votes and potentially flips
        for (i, node) in nodes.iter_mut().enumerate() {
            let mut registry = node.app_objects.write().await;
            let ids = registry.ids();
            for id in ids {
                if let Some(obj) = registry.objects.get_mut(&id) {
                    if let Some(slush) = obj.as_any_mut().downcast_mut::<SlushObject>() {
                        let flipped = slush.process_round(round);
                        slush.current_round = round;
                        if flipped {
                            println!(
                                "  [node-{i}] Round {round}: flipped to {}",
                                slush.color.unwrap()
                            );
                        }
                    }
                }
            }
        }
    }

    // Finalize
    println!("\n=== Slush consensus complete ===");
    for (i, node) in nodes.iter_mut().enumerate() {
        let mut registry = node.app_objects.write().await;
        let ids = registry.ids();
        for id in ids {
            if let Some(obj) = registry.objects.get_mut(&id) {
                if let Some(slush) = obj.as_any_mut().downcast_mut::<SlushObject>() {
                    slush.finalize();
                    println!(
                        "  node-{i}: accepted={}",
                        slush.accepted.map(|c| c.to_string()).unwrap_or("?".into())
                    );
                }
            }
        }
    }

    for mut node in nodes {
        node.close().await?;
    }

    Ok(())
}
