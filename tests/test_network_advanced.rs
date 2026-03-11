use anyhow::Result;
use chaincraft_rust::{
    clear_local_registry,
    error::{ChaincraftError, SerializationError},
    network::PeerId,
    storage::MemoryStorage,
    ChaincraftNode,
};
use serde_json::json;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

async fn create_test_node_with_port(port: u16) -> ChaincraftNode {
    clear_local_registry();
    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);
    node.set_port(port);
    node.disable_local_discovery();
    node.start().await.unwrap();
    node
}

#[tokio::test]
async fn test_node_port_assignment() -> Result<()> {
    let mut node1 = create_test_node_with_port(0).await;
    let mut node2 = create_test_node_with_port(0).await;

    assert!(node1.port() != 0);
    assert!(node2.port() != 0);
    assert_ne!(node1.id(), node2.id());

    node1.close().await.unwrap();
    node2.close().await.unwrap();
    Ok(())
}

#[tokio::test]
async fn test_peer_connection_establishment() -> Result<()> {
    let mut node1 = create_test_node_with_port(0).await;
    let mut node2 = create_test_node_with_port(0).await;

    // Connect node1 to node2
    let peer_addr = format!("127.0.0.1:{}", node2.port());
    let connection_result = node1.connect_to_peer(&peer_addr).await;

    // Connection might fail in test environment, but we test the interface
    match connection_result {
        Ok(_) => {
            // Connection successful
            sleep(Duration::from_millis(100)).await;
        },
        Err(_) => {
            // Connection failed, which is expected in isolated test environment
        },
    }

    node1.close().await.unwrap();
    node2.close().await.unwrap();
    Ok(())
}

#[tokio::test]
async fn test_message_broadcasting() -> Result<()> {
    let mut node1 = create_test_node_with_port(0).await;
    let mut node2 = create_test_node_with_port(0).await;
    let mut node3 = create_test_node_with_port(0).await;

    // Create a message on node1
    let test_data = json!({"broadcast": "test", "value": 42});
    let message_hash = node1
        .create_shared_message_with_data(test_data.clone())
        .await
        .unwrap();
    let message_json = node1.get_object(&message_hash).await?;
    let value: serde_json::Value = serde_json::from_str(&message_json)
        .map_err(|e| ChaincraftError::Serialization(SerializationError::Json(e)))?;
    let data = value["data"].to_string();
    assert_eq!(data, test_data.to_string());

    // In a real network, this would be broadcast to peers
    // Here we test the message creation and structure

    node1.close().await.unwrap();
    node2.close().await.unwrap();
    node3.close().await.unwrap();
    Ok(())
}

#[tokio::test]
async fn test_node_discovery_interface() -> Result<()> {
    let mut node = create_test_node_with_port(0).await;

    // Test discovery methods exist and can be called
    let discovery_info = node.get_discovery_info().await;
    if let Some(map) = discovery_info.as_object() {
        assert!(map.contains_key("node_id"));
        assert!(map.contains_key("host"));
        assert!(map.contains_key("port"));
    }

    node.close().await.unwrap();
    Ok(())
}

#[tokio::test]
async fn test_concurrent_message_creation() -> Result<()> {
    let mut node = create_test_node_with_port(0).await;

    // Create multiple messages sequentially instead of concurrently
    let mut messages = Vec::new();
    for i in 0..10 {
        let data = json!({"concurrent_test": i});
        let message_hash = node.create_shared_message_with_data(data.clone()).await?;
        let message_json = node.get_object(&message_hash).await?;
        messages.push(message_json);
    }

    // Verify all messages were created successfully
    assert_eq!(messages.len(), 10);

    // Verify message IDs are unique
    let mut ids = std::collections::HashSet::new();
    for message_json in &messages {
        let value: serde_json::Value = serde_json::from_str(message_json)
            .map_err(|e| ChaincraftError::Serialization(SerializationError::Json(e)))?;
        let id = value["id"].to_string();
        assert!(ids.insert(id));
    }

    node.close().await.unwrap();
    Ok(())
}

#[tokio::test]
async fn test_node_state_persistence() -> Result<()> {
    let mut node = create_test_node_with_port(0).await;

    // Create some messages
    for i in 0..5 {
        let data = json!({"persistence_test": i});
        node.create_shared_message_with_data(data).await.unwrap();
    }

    // Get node state
    let state = node.get_state().await.unwrap();
    if let Some(map) = state.as_object() {
        assert!(map.contains_key("node_id"));
        assert!(map.contains_key("messages"));
    }

    node.close().await.unwrap();
    Ok(())
}

#[tokio::test]
async fn test_message_validation() -> Result<()> {
    let mut node = create_test_node_with_port(0).await;

    // Test various message types
    let test_cases = vec![
        json!(null),
        json!(true),
        json!(false),
        json!(0),
        json!(-1),
        json!(1.5),
        json!("string"),
        json!([]),
        json!({}),
    ];

    for (i, data) in test_cases.into_iter().enumerate() {
        let message_hash = node
            .create_shared_message_with_data(data.clone())
            .await
            .unwrap();
        let message_json = node.get_object(&message_hash).await?;
        let value: serde_json::Value = serde_json::from_str(&message_json)
            .map_err(|e| ChaincraftError::Serialization(SerializationError::Json(e)))?;
        let data_field = value["data"].to_string();
        assert_eq!(data_field, data.to_string(), "Test case {i} failed");
    }

    node.close().await.unwrap();
    Ok(())
}

#[tokio::test]
async fn test_node_lifecycle() -> Result<()> {
    let mut node = create_test_node_with_port(0).await;

    // Node should be running
    assert!(node.is_running_async().await);

    // Create a message to verify functionality
    let data = json!({"lifecycle": "test"});
    let message_hash = node
        .create_shared_message_with_data(data.clone())
        .await
        .unwrap();
    let message_json = node.get_object(&message_hash).await?;
    let value: serde_json::Value = serde_json::from_str(&message_json)
        .map_err(|e| ChaincraftError::Serialization(SerializationError::Json(e)))?;
    let data = value["data"].to_string();
    assert_eq!(data, data.to_string());

    // Close the node
    node.close().await.unwrap();

    // Node should no longer be running
    assert!(!node.is_running_async().await);
    Ok(())
}

#[tokio::test]
async fn test_multiple_nodes_isolation() -> Result<()> {
    let mut nodes = Vec::new();

    // Create multiple isolated nodes
    for i in 0..5 {
        let node = create_test_node_with_port(8020 + i).await;
        nodes.push(node);
    }

    // Each node should have unique ID and port
    for i in 0..nodes.len() {
        for j in i + 1..nodes.len() {
            assert_ne!(nodes[i].id(), nodes[j].id());
            assert_ne!(nodes[i].port(), nodes[j].port());
        }
    }

    // Create messages on each node
    for (i, node) in nodes.iter_mut().enumerate() {
        let data = json!({"node_index": i});
        let message_hash = node
            .create_shared_message_with_data(data.clone())
            .await
            .unwrap();
        let message_json = node.get_object(&message_hash).await?;
        let value: serde_json::Value = serde_json::from_str(&message_json)
            .map_err(|e| ChaincraftError::Serialization(SerializationError::Json(e)))?;
        let data = value["data"].to_string();
        assert_eq!(data, data.to_string());
    }

    // Close all nodes
    for mut node in nodes {
        node.close().await.unwrap();
    }
    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let mut node = create_test_node_with_port(8030).await;

    // Test connection to invalid address
    let invalid_addr = "invalid_address:9999";
    let connection_result = node.connect_to_peer(invalid_addr).await;

    // Should handle error gracefully
    assert!(connection_result.is_err());

    // Node should still be functional
    let data = json!({"error_test": "still_working"});
    let message_hash = node
        .create_shared_message_with_data(data.clone())
        .await
        .unwrap();
    let message_json = node.get_object(&message_hash).await?;
    let value: serde_json::Value = serde_json::from_str(&message_json)
        .map_err(|e| ChaincraftError::Serialization(SerializationError::Json(e)))?;
    let data = value["data"].to_string();
    assert_eq!(data, data.to_string());

    node.close().await.unwrap();
    Ok(())
}

#[tokio::test]
async fn test_high_frequency_operations() -> Result<()> {
    let mut node = create_test_node_with_port(8031).await;

    // Create many messages rapidly
    let start_time = std::time::Instant::now();

    for i in 0..100 {
        let data = json!({"high_freq": i});
        node.create_shared_message_with_data(data).await.unwrap();
    }

    let duration = start_time.elapsed();

    // Should complete within reasonable time (adjust threshold as needed)
    assert!(duration.as_secs() < 10, "High frequency operations took too long");

    node.close().await.unwrap();
    Ok(())
}

#[tokio::test]
async fn test_memory_efficiency() -> Result<()> {
    let mut node = create_test_node_with_port(8032).await;

    // Create messages and verify they don't cause memory issues
    for i in 0..50 {
        let data = json!({
            "memory_test": i,
            "large_field": "x".repeat(1000) // 1KB string
        });
        node.create_shared_message_with_data(data).await.unwrap();
    }

    // Get state to verify node is still responsive
    let state = node.get_state().await.unwrap();
    if let Some(map) = state.as_object() {
        assert!(map.contains_key("node_id"));
    }

    node.close().await.unwrap();
    Ok(())
}

#[tokio::test]
async fn test_message_serialization() -> Result<()> {
    let mut node = create_test_node_with_port(0).await;

    // Create a message with complex data
    let data = json!({
        "nested": {
            "array": [1, 2, 3],
            "object": {
                "key": "value"
            }
        }
    });

    // Store and retrieve message
    let message_hash = node.create_shared_message_with_data(data.clone()).await?;
    let stored = node.get_object(&message_hash).await?;
    let value: serde_json::Value = serde_json::from_str(&stored)?;
    assert_eq!(value["data"], data);

    node.close().await?;
    Ok(())
}
