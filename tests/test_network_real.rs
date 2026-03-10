use anyhow::Result;
use chaincraft_rust::{
    network::PeerId,
    storage::MemoryStorage,
    ChaincraftNode,
};
use serde_json::json;
use std::sync::Arc;
use tokio::time::{sleep, Duration, Instant};

async fn create_node_with_port(port: u16) -> ChaincraftNode {
    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);
    node.set_port(port);
    node.start().await.unwrap();
    node
}

/// Wait until a node has a given object hash or timeout.
async fn wait_for_object(node: &ChaincraftNode, hash: &str, timeout: Duration) -> bool {
    let start = Instant::now();
    loop {
        if start.elapsed() > timeout {
            return false;
        }
        if node.get_object(hash).await.is_ok() {
            return true;
        }
        sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn test_message_propagation_three_nodes() -> Result<()> {
    // Use high ports unlikely to clash with other tests
    let mut node1 = create_node_with_port(9001).await;
    let mut node2 = create_node_with_port(9002).await;
    let mut node3 = create_node_with_port(9003).await;

    // Connect node1 to node2 and node3 (unidirectional is enough for UDP broadcast)
    let addr2 = format!("127.0.0.1:{}", node2.port());
    let addr3 = format!("127.0.0.1:{}", node3.port());
    node1.connect_to_peer(&addr2).await.unwrap();
    node1.connect_to_peer(&addr3).await.unwrap();

    // Give the networking layer a moment to settle
    sleep(Duration::from_millis(100)).await;

    // Create a shared message on node1
    let payload = json!({"network": "propagation_test", "value": 123});
    let hash = node1
        .create_shared_message_with_data(payload.clone())
        .await
        .unwrap();

    // Verify it exists locally
    let raw = node1.get_object(&hash).await?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    assert_eq!(value["data"], payload);

    // Wait for propagation to node2 and node3
    let timeout = Duration::from_secs(5);
    assert!(
        wait_for_object(&node2, &hash, timeout).await,
        "message did not reach node2 in time"
    );
    assert!(
        wait_for_object(&node3, &hash, timeout).await,
        "message did not reach node3 in time"
    );

    // Confirm payload integrity on node2/node3
    let raw2 = node2.get_object(&hash).await?;
    let v2: serde_json::Value = serde_json::from_str(&raw2)?;
    assert_eq!(v2["data"], payload);

    let raw3 = node3.get_object(&hash).await?;
    let v3: serde_json::Value = serde_json::from_str(&raw3)?;
    assert_eq!(v3["data"], payload);

    node1.close().await.unwrap();
    node2.close().await.unwrap();
    node3.close().await.unwrap();
    Ok(())
}

