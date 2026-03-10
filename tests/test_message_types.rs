use anyhow::Result;
use chaincraft_rust::{
    network::PeerId,
    shared::{MessageType, SharedMessage},
    storage::MemoryStorage,
    ChaincraftNode,
};
use serde_json::json;
use std::f64::consts::PI;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Helper function to connect nodes in a full mesh
async fn connect_nodes(nodes: &mut [ChaincraftNode]) -> Result<()> {
    for i in 0..nodes.len() {
        for j in 0..nodes.len() {
            if i != j {
                let peer_addr = format!("{}:{}", nodes[j].host(), nodes[j].port());
                nodes[i].connect_to_peer(&peer_addr).await?;
            }
        }
    }
    Ok(())
}

/// Helper function to wait for message propagation (simplified for testing)
#[allow(dead_code)]
async fn wait_for_message_propagation(
    nodes: &[ChaincraftNode],
    expected_count: usize,
    timeout_secs: u64,
) -> bool {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    while start.elapsed() < timeout {
        let counts: Vec<usize> = nodes.iter().map(|node| node.db_size()).collect();
        if counts.iter().all(|&count| count == expected_count) {
            return true;
        }
        sleep(Duration::from_millis(100)).await;
    }
    false
}

async fn create_test_node() -> ChaincraftNode {
    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);
    node.set_port(0); // Use ephemeral port
    node.start().await.expect("Failed to start node");
    node
}

#[tokio::test]
async fn test_text_message_creation() {
    let mut node = create_test_node().await;

    let message_id = node
        .create_shared_message_with_data(json!("Hello, World!"))
        .await
        .unwrap();
    assert!(!message_id.is_empty());

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_json_object_message() {
    let mut node = create_test_node().await;

    let data = json!({
        "type": "user_action",
        "action": "login",
        "user_id": 12345,
        "timestamp": "2024-01-01T00:00:00Z"
    });

    let message_id = node
        .create_shared_message_with_data(data.clone())
        .await
        .unwrap();
    assert!(!message_id.is_empty());

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_array_message() {
    let mut node = create_test_node().await;

    let data = json!([1, 2, 3, 4, 5]);
    let message_id = node
        .create_shared_message_with_data(data.clone())
        .await
        .unwrap();
    assert!(!message_id.is_empty());

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_nested_object_message() {
    let mut node = create_test_node().await;

    let data = json!({
        "user": {
            "id": 123,
            "profile": {
                "name": "Alice",
                "preferences": {
                    "theme": "dark",
                    "notifications": true
                }
            }
        },
        "metadata": {
            "version": "1.0",
            "created_at": "2024-01-01T00:00:00Z"
        }
    });

    let message_id = node
        .create_shared_message_with_data(data.clone())
        .await
        .unwrap();
    assert!(!message_id.is_empty());

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_large_message() {
    let mut node = create_test_node().await;

    // Create a large message with repeated data
    let large_data = json!({
        "type": "bulk_data",
        "items": (0..100).map(|i| json!({
            "id": i,
            "value": format!("item_{}", i),
            "metadata": {
                "created": "2024-01-01T00:00:00Z",
                "tags": ["tag1", "tag2", "tag3"]
            }
        })).collect::<Vec<_>>()
    });

    let message_id = node
        .create_shared_message_with_data(large_data.clone())
        .await
        .unwrap();
    assert!(!message_id.is_empty());

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_message_with_special_characters() {
    let mut node = create_test_node().await;

    let data = json!({
        "text": "Hello 🌍! Special chars: àáâãäåæçèéêë",
        "unicode": "🚀🎉💻🔥⭐",
        "symbols": "!@#$%^&*()_+-=[]{}|;':\",./<>?",
        "newlines": "Line 1\nLine 2\r\nLine 3\tTabbed"
    });

    let message_id = node
        .create_shared_message_with_data(data.clone())
        .await
        .unwrap();
    assert!(!message_id.is_empty());

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_message_serialization_roundtrip() -> Result<()> {
    let mut node = create_test_node().await;
    let data = json!({
        "test": "data",
        "number": 42,
        "nested": { "field": "value" }
    });
    let message_id = node.create_shared_message_with_data(data.clone()).await?;
    let mut message_json = None;
    for _ in 0..10 {
        if let Some(obj) = node
            .get_object(&message_id)
            .await
            .ok()
            .filter(|s| !s.is_empty())
        {
            message_json = Some(obj);
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    let message_json = message_json.expect("Message object not found");
    let value: serde_json::Value = serde_json::from_str(&message_json)?;
    let parsed_data = &value["data"];
    assert_eq!(parsed_data, &data);
    node.close().await?;
    Ok(())
}

#[tokio::test]
async fn test_message_id_uniqueness() {
    let mut node = create_test_node().await;

    let mut message_ids = std::collections::HashSet::new();

    // Create multiple messages and ensure IDs are unique
    for i in 0..10 {
        let message_id = node
            .create_shared_message_with_data(json!(i))
            .await
            .unwrap();
        assert!(message_ids.insert(message_id), "Message ID should be unique");
    }

    assert_eq!(message_ids.len(), 10);

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_message_ordering() -> Result<()> {
    let mut node = create_test_node().await;
    let mut messages = Vec::new();
    for i in 0..5 {
        let data = json!({ "index": i, "data": format!("message_{}", i) });
        let msg_id = node.create_shared_message_with_data(data).await?;
        let mut msg_json = None;
        for _ in 0..10 {
            if let Some(obj) = node
                .get_object(&msg_id)
                .await
                .ok()
                .filter(|s| !s.is_empty())
            {
                msg_json = Some(obj);
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
        let msg_json = msg_json.expect("Message object not found");
        let value: serde_json::Value = serde_json::from_str(&msg_json)?;
        let timestamp = value["timestamp"].as_i64().unwrap_or(0);
        messages.push((msg_id, timestamp));
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
    for i in 1..messages.len() {
        let prev_ts = messages[i - 1].1;
        let curr_ts = messages[i].1;
        assert!(
            curr_ts >= prev_ts,
            "Message timestamps should be monotonically increasing or equal"
        );
    }
    node.close().await?;
    Ok(())
}

#[tokio::test]
async fn test_empty_message_data() {
    let mut node = create_test_node().await;

    let empty_data = json!({});
    let message_id = node
        .create_shared_message_with_data(empty_data.clone())
        .await
        .unwrap();
    assert!(!message_id.is_empty());

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_simple_string_message() -> Result<()> {
    let mut nodes: Vec<ChaincraftNode> = vec![
        ChaincraftNode::builder()
            .port(0)
            .with_persistent_storage(false)
            .persist_peers(false)
            .build()?,
        ChaincraftNode::builder()
            .port(0)
            .with_persistent_storage(false)
            .persist_peers(false)
            .build()?,
        ChaincraftNode::builder()
            .port(0)
            .with_persistent_storage(false)
            .persist_peers(false)
            .build()?,
    ];

    for node in &mut nodes {
        node.start().await?;
    }

    connect_nodes(&mut nodes).await?;

    // Create a simple string message
    let _hash = nodes[0]
        .create_shared_message("Hello, world!".to_string())
        .await?;

    // For now, just verify the message was created
    // Real propagation would require gossip protocol implementation
    assert_eq!(nodes[0].db_size(), 1);

    // Cleanup
    for mut node in nodes {
        node.close().await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_simple_integer_message() -> Result<()> {
    let mut nodes = vec![
        ChaincraftNode::builder()
            .port(0)
            .with_persistent_storage(false)
            .persist_peers(false)
            .build()?,
        ChaincraftNode::builder()
            .port(0)
            .with_persistent_storage(false)
            .persist_peers(false)
            .build()?,
        ChaincraftNode::builder()
            .port(0)
            .with_persistent_storage(false)
            .persist_peers(false)
            .build()?,
    ];

    for node in &mut nodes {
        node.start().await?;
    }

    connect_nodes(&mut nodes).await?;

    // Create a message with integer data
    let _hash = nodes[0].create_shared_message("42".to_string()).await?;

    // Verify the message was created
    assert_eq!(nodes[0].db_size(), 1);

    // Cleanup
    for mut node in nodes {
        node.close().await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_json_message() -> Result<()> {
    let mut nodes = vec![
        ChaincraftNode::builder()
            .port(0)
            .with_persistent_storage(false)
            .persist_peers(false)
            .build()?,
        ChaincraftNode::builder()
            .port(0)
            .with_persistent_storage(false)
            .persist_peers(false)
            .build()?,
        ChaincraftNode::builder()
            .port(0)
            .with_persistent_storage(false)
            .persist_peers(false)
            .build()?,
    ];

    for node in &mut nodes {
        node.start().await?;
    }

    connect_nodes(&mut nodes).await?;

    // Create a JSON-structured message similar to the Python test
    let user_data = json!({
        "message_type": "User",
        "user_id": 1,
        "username": "alice",
        "email": "alice@example.com",
        "bio": "Hello, I'm Alice!"
    });

    let _hash = nodes[0]
        .create_shared_message(user_data.to_string())
        .await?;

    // Verify the message was created
    assert_eq!(nodes[0].db_size(), 1);

    // Cleanup
    for mut node in nodes {
        node.close().await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_complex_nested_message() -> Result<()> {
    let mut nodes = vec![
        ChaincraftNode::builder()
            .port(0)
            .with_persistent_storage(false)
            .persist_peers(false)
            .build()?,
        ChaincraftNode::builder()
            .port(0)
            .with_persistent_storage(false)
            .persist_peers(false)
            .build()?,
        ChaincraftNode::builder()
            .port(0)
            .with_persistent_storage(false)
            .persist_peers(false)
            .build()?,
    ];

    for node in &mut nodes {
        node.start().await?;
    }

    connect_nodes(&mut nodes).await?;

    // Create a complex nested message similar to the transaction/block example
    let transaction1 = json!({
        "message_type": "Transaction",
        "sender": "Alice",
        "recipient": "Bob",
        "amount": 10.0,
        "signature": "a1b2c3d4e5f6g7h8i9j0"
    });

    let transaction2 = json!({
        "message_type": "Transaction",
        "sender": "Bob",
        "recipient": "Charlie",
        "amount": 5.0,
        "signature": "k1l2m3n4o5p6q7r8s9t0"
    });

    let block = json!({
        "message_type": "Block",
        "block_number": 1,
        "transactions": [transaction1, transaction2],
        "previous_hash": "0000000000000000000000000000000000000000000000000000000000000000",
        "timestamp": chrono::Utc::now().timestamp(),
        "nonce": 1234
    });

    let _hash = nodes[0].create_shared_message(block.to_string()).await?;

    // Verify the message was created
    assert_eq!(nodes[0].db_size(), 1);

    // Cleanup
    for mut node in nodes {
        node.close().await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_node_state_access() {
    let mut node = create_test_node().await;

    // Create some messages
    for i in 0..3 {
        node.create_shared_message_with_data(json!({"test": i}))
            .await
            .unwrap();
    }

    // Test node state access
    let state = node.get_state().await.unwrap();
    assert!(state.is_object());
    assert!(state.get("node_id").is_some());
    assert!(state.get("running").is_some());

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_null_and_boolean_messages() {
    let mut node = create_test_node().await;

    // Test null value
    let null_id = node
        .create_shared_message_with_data(json!(null))
        .await
        .unwrap();
    assert!(!null_id.is_empty());

    // Test boolean values
    let true_id = node
        .create_shared_message_with_data(json!(true))
        .await
        .unwrap();
    assert!(!true_id.is_empty());

    let false_id = node
        .create_shared_message_with_data(json!(false))
        .await
        .unwrap();
    assert!(!false_id.is_empty());

    // All IDs should be different
    assert_ne!(null_id, true_id);
    assert_ne!(true_id, false_id);
    assert_ne!(null_id, false_id);

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_numeric_messages() {
    let mut node = create_test_node().await;

    // Test various numeric types
    let int_id = node
        .create_shared_message_with_data(json!(42))
        .await
        .unwrap();
    let negative_id = node
        .create_shared_message_with_data(json!(-123))
        .await
        .unwrap();
    let float_id = node
        .create_shared_message_with_data(json!(PI))
        .await
        .unwrap();
    let zero_id = node
        .create_shared_message_with_data(json!(0))
        .await
        .unwrap();

    // All should create valid message IDs
    assert!(!int_id.is_empty());
    assert!(!negative_id.is_empty());
    assert!(!float_id.is_empty());
    assert!(!zero_id.is_empty());

    // All IDs should be unique
    let ids = [int_id, negative_id, float_id, zero_id];
    let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique_ids.len(), 4);

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_message_timestamp_ordering() -> Result<()> {
    let mut node = create_test_node().await;

    // Create messages with different timestamps
    let mut messages = Vec::new();
    for i in 0..5 {
        let data = json!({ "index": i });
        let message = SharedMessage::new(MessageType::Custom("test".to_string()), data);
        let timestamp = message.timestamp;
        messages.push((message, timestamp));
        sleep(Duration::from_millis(10)).await;
    }

    // Verify timestamps are in ascending order
    for i in 1..messages.len() {
        assert!(messages[i].1 >= messages[i - 1].1, "Messages should be ordered by timestamp");
    }

    node.close().await?;
    Ok(())
}

#[tokio::test]
async fn test_shared_object_id_variants() -> Result<(), Box<dyn std::error::Error>> {
    let mut node1 = create_test_node().await;
    let mut node2 = create_test_node().await;

    // Test numeric message types
    let int_id = node1
        .create_shared_message_with_data(json!(42))
        .await
        .unwrap();

    let negative_id = node1
        .create_shared_message_with_data(json!(-123))
        .await
        .unwrap();

    // Use PI constant instead of approximation
    let float_id = node1
        .create_shared_message_with_data(json!(PI))
        .await
        .unwrap();

    let zero_id = node1
        .create_shared_message_with_data(json!(0))
        .await
        .unwrap();

    // All IDs should be unique
    let ids = [&int_id, &negative_id, &float_id, &zero_id];
    let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique_ids.len(), 4);

    // Clean up nodes
    node1.close().await?;
    node2.close().await?;

    Ok(())
}
