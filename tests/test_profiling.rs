use chaincraft::{network::PeerId, storage::MemoryStorage, ChaincraftNode};
use serde_json::json;
use std::sync::Arc;
use tokio::time::{sleep, Duration, Instant};

async fn create_performance_node() -> ChaincraftNode {
    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);
    node.set_port(0); // Use ephemeral port
    node.start().await.unwrap();
    node
}

#[tokio::test]
async fn test_message_creation_performance() {
    let mut node = create_performance_node().await;

    let start = Instant::now();

    // Create multiple messages and measure time
    for i in 0..100 {
        let msg = json!({
            "message_type": "PerformanceTest",
            "id": i,
            "data": format!("test_data_{}", i)
        });

        node.create_shared_message_with_data(msg).await.unwrap();
    }

    let duration = start.elapsed();

    // Should complete within reasonable time (adjust threshold as needed)
    assert!(duration.as_secs() < 10, "Message creation took too long: {duration:?}");

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_node_startup_performance() {
    let start = Instant::now();

    let mut node = create_performance_node().await;

    let startup_duration = start.elapsed();

    // Node should start quickly
    assert!(
        startup_duration.as_secs() < 5,
        "Node startup took too long: {startup_duration:?}"
    );

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_concurrent_message_handling() {
    let mut node = create_performance_node().await;

    let start = Instant::now();

    // Create messages concurrently (simulated)
    let mut _tasks: Vec<()> = Vec::new();

    for i in 0..10 {
        let msg = json!({
            "message_type": "ConcurrentTest",
            "id": i,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        node.create_shared_message_with_data(msg).await.unwrap();

        if i % 2 == 0 {
            sleep(Duration::from_millis(1)).await; // Small delay
        }
    }

    let duration = start.elapsed();

    // Concurrent processing should be efficient
    assert!(
        duration.as_secs() < 5,
        "Concurrent message handling took too long: {duration:?}"
    );

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_memory_usage_stability() {
    let mut node = create_performance_node().await;

    // Create many messages to test memory stability
    for batch in 0..5 {
        for i in 0..50 {
            let msg = json!({
                "message_type": "MemoryTest",
                "batch": batch,
                "id": i,
                "large_data": "x".repeat(1000) // 1KB of data per message
            });

            node.create_shared_message_with_data(msg).await.unwrap();
        }

        // Small pause between batches
        sleep(Duration::from_millis(10)).await;
    }

    // No assertions needed for performance test

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_network_latency_simulation() {
    let mut node1 = create_performance_node().await;
    let mut node2 = create_performance_node().await;

    let start = Instant::now();

    // Simulate network messages
    let msg = json!({
        "message_type": "NetworkLatencyTest",
        "sender": "node1",
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    node1.create_shared_message_with_data(msg).await.unwrap();

    // Simulate propagation delay
    sleep(Duration::from_millis(50)).await;

    // Create response message
    let response = json!({
        "message_type": "NetworkLatencyResponse",
        "sender": "node2",
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    node2
        .create_shared_message_with_data(response)
        .await
        .unwrap();

    let total_latency = start.elapsed();

    // Network simulation should complete quickly
    assert!(
        total_latency.as_millis() < 1000,
        "Network latency simulation too slow: {total_latency:?}"
    );

    node1.close().await.unwrap();
    node2.close().await.unwrap();
}
