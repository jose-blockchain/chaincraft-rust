//! Randomness Beacon Example
//!
//! Demonstrates the verifiable randomness beacon protocol.
//! Ported from the Python chaincraft examples (randomness_beacon.py).
//!
//! Features:
//! - Validator registration
//! - VRF proof submission
//! - Threshold-based randomness finalization
//!
//! Run with: `cargo run --example randomness_beacon_example`

use chaincraft_rust::{
    error::Result,
    examples::randomness_beacon::{helpers, RandomnessBeaconObject},
    network::PeerId,
    shared_object::ApplicationObject,
    storage::MemoryStorage,
    ChaincraftNode,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    println!("Chaincraft Randomness Beacon Example");
    println!("=====================================\n");

    let beacon = RandomnessBeaconObject::new(60, 2)?;
    let my_address = beacon.my_validator_address.clone();

    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);

    let beacon_obj: Box<dyn ApplicationObject> = Box::new(beacon);
    node.add_shared_object(beacon_obj).await?;
    node.start().await?;

    println!(
        "Node started. Validator address: {}...",
        &my_address[..my_address.len().min(40)]
    );

    let reg_signer = chaincraft_rust::crypto::ecdsa::ECDSASigner::new()?;
    let reg_msg = helpers::create_validator_registration(
        my_address.clone(),
        reg_signer.get_public_key_pem()?,
        "vrf_key_placeholder".to_string(),
        100,
        &reg_signer,
    )?;

    node.create_shared_message_with_data(reg_msg).await?;
    println!("Validator registration sent");
    sleep(Duration::from_millis(200)).await;

    let shared_objects = node.shared_objects().await;
    if let Some(obj) = shared_objects.first() {
        if obj.type_name() == "RandomnessBeacon" {
            let state = obj.get_state().await?;
            println!("\nBeacon state: {}", serde_json::to_string_pretty(&state).unwrap());
        }
    }

    println!("\nShutting down...");
    node.close().await?;
    println!("Done.");

    Ok(())
}
