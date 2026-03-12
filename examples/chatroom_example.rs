//! Chatroom Example
//!
//! Demonstrates the decentralized chatroom protocol built on Chaincraft.
//! Ported from the Python chaincraft examples (chatroom_protocol.py).
//!
//! Features:
//! - Create chatrooms with admin control
//! - Post messages to chatrooms
//! - ECDSA-signed messages for authentication
//!
//! Run with: `cargo run --example chatroom_example`

use chaincraft::{
    crypto::ecdsa::ECDSASigner,
    error::Result,
    examples::chatroom::{helpers, ChatroomObject},
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

    println!("Chaincraft Chatroom Example");
    println!("===========================\n");

    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);

    let chatroom_obj: Box<dyn ApplicationObject> = Box::new(ChatroomObject::new());
    node.add_shared_object(chatroom_obj).await?;
    node.start().await?;

    println!("Node started with peer ID: {}", node.id());

    let admin_signer = ECDSASigner::new()?;

    let create_msg = helpers::create_chatroom_message("rust_chatroom".to_string(), &admin_signer)?;
    node.create_shared_message_with_data(create_msg).await?;

    println!("Created chatroom 'rust_chatroom'");
    sleep(Duration::from_millis(200)).await;

    let post_msg = helpers::create_post_message(
        "rust_chatroom".to_string(),
        "Hello from Chaincraft Rust!".to_string(),
        &admin_signer,
    )?;
    node.create_shared_message_with_data(post_msg).await?;

    println!("Posted message to chatroom");
    sleep(Duration::from_millis(200)).await;

    let shared_objects = node.shared_objects().await;
    if let Some(obj) = shared_objects.first() {
        if let Some(chatroom_obj) = obj.as_any().downcast_ref::<ChatroomObject>() {
            let chatrooms = chatroom_obj.get_chatrooms();
            println!("\nChatrooms: {:?}", chatrooms.keys().collect::<Vec<_>>());
            if let Some(room) = chatrooms.get("rust_chatroom") {
                println!("Messages in rust_chatroom: {}", room.messages.len());
            }
        }
    }

    println!("\nShutting down...");
    node.close().await?;
    println!("Done.");

    Ok(())
}
