//! Integration tests for runnable examples.
//!
//! Verifies that example logic compiles, runs, and produces expected results.

use chaincraft_rust::{
    crypto::{ecdsa::ECDSASigner, pow::PoWChallenge, KeylessCryptoPrimitive, ProofOfWork},
    examples::chatroom::{helpers, ChatroomObject},
    network::PeerId,
    shared_object::{ApplicationObject, SimpleSharedNumber},
    storage::MemoryStorage,
    ChaincraftNode,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

// -----------------------------------------------------------------------------
// Chatroom Example Tests
// -----------------------------------------------------------------------------

async fn create_chatroom_node() -> ChaincraftNode {
    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);
    let chatroom_obj: Box<dyn ApplicationObject> = Box::new(ChatroomObject::new());
    node.add_shared_object(chatroom_obj).await.unwrap();
    node.start().await.unwrap();
    node
}

#[tokio::test]
async fn test_chatroom_example_creates_room_and_posts() {
    let mut node = create_chatroom_node().await;
    let signer = ECDSASigner::new().unwrap();

    let create_msg = helpers::create_chatroom_message("example_room".to_string(), &signer).unwrap();
    node.create_shared_message_with_data(create_msg)
        .await
        .unwrap();
    sleep(Duration::from_millis(100)).await;

    let post_msg = helpers::create_post_message(
        "example_room".to_string(),
        "Test message".to_string(),
        &signer,
    )
    .unwrap();
    node.create_shared_message_with_data(post_msg)
        .await
        .unwrap();
    sleep(Duration::from_millis(100)).await;

    let objects = node.shared_objects().await;
    let chatroom = objects
        .first()
        .and_then(|o| o.as_any().downcast_ref::<ChatroomObject>());
    assert!(chatroom.is_some());
    let room = chatroom.unwrap().get_chatroom("example_room").unwrap();
    assert_eq!(room.messages.len(), 1);

    node.close().await.unwrap();
}

// -----------------------------------------------------------------------------
// Shared Objects Example Tests
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_shared_objects_example_network() {
    let mut nodes = Vec::new();
    for _ in 0..3 {
        let id = PeerId::new();
        let storage = Arc::new(MemoryStorage::new());
        let mut node = ChaincraftNode::new(id, storage);
        let shared: Box<dyn ApplicationObject> = Box::new(SimpleSharedNumber::new());
        node.add_shared_object(shared).await.unwrap();
        node.start().await.unwrap();
        nodes.push(node);
    }

    for i in 0..3 {
        let next = (i + 1) % 3;
        let addr = format!("{}:{}", nodes[next].host(), nodes[next].port());
        nodes[i].connect_to_peer(&addr).await.unwrap();
    }

    sleep(Duration::from_millis(500)).await;

    nodes[0]
        .create_shared_message_with_data(serde_json::json!(42))
        .await
        .unwrap();
    sleep(Duration::from_millis(300)).await;

    assert_eq!(nodes.len(), 3);
    for mut node in nodes {
        node.close().await.unwrap();
    }
}

// -----------------------------------------------------------------------------
// Randomness Beacon Example Tests
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_randomness_beacon_example_starts() {
    let beacon = chaincraft_rust::examples::randomness_beacon::RandomnessBeaconObject::new(60, 2)
        .unwrap();
    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);
    let beacon_obj: Box<dyn ApplicationObject> = Box::new(beacon);
    node.add_shared_object(beacon_obj).await.unwrap();
    node.start().await.unwrap();
    assert!(node.shared_object_count().await > 0);
    node.close().await.unwrap();
}

// -----------------------------------------------------------------------------
// Proof of Work Example Tests
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_proof_of_work_example_mines_and_verifies() {
    let pow = ProofOfWork::with_difficulty(2);
    let challenge = PoWChallenge::new("test_challenge");

    let proof = pow.create_proof(challenge.clone()).await.unwrap();
    assert!(proof.hash.starts_with("00"));
    assert!(proof.hash.len() == 64);

    let verified = pow.verify_proof(challenge, proof).await.unwrap();
    assert!(verified);
}
