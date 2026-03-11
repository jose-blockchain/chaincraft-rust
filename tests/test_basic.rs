use chaincraft_rust::{error::Result, ChaincraftNode};
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Helper function to create a network of nodes with a base port
async fn create_network_with_base_port(
    num_nodes: usize,
    _reset_db: bool,
    base_port: u16,
) -> Result<Vec<ChaincraftNode>> {
    let mut nodes = Vec::new();
    for i in 0..num_nodes {
        let mut node = ChaincraftNode::builder()
            .with_persistent_storage(false)
            .build()?;
        // Ensure each node binds to a unique port to avoid conflicts
        let port = base_port + i as u16;
        node.set_port(port);
        nodes.push(node);
    }
    Ok(nodes)
}

/// Helper function to connect nodes randomly
async fn connect_nodes(nodes: &mut [ChaincraftNode]) -> Result<()> {
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    let mut rng = thread_rng();
    let node_count = nodes.len();

    // Create connection pairs first to avoid borrowing issues
    let mut connections = Vec::new();
    for i in 0..node_count {
        let mut other_indices: Vec<usize> = (0..node_count).filter(|&x| x != i).collect();
        other_indices.shuffle(&mut rng);

        for &j in other_indices.iter().take(3) {
            connections.push((i, j));
        }
    }

    // Now make the connections
    for (i, j) in connections {
        let current_peer_count = nodes[i].get_peers().await.len();
        if current_peer_count < nodes[i].max_peers() {
            let peer_addr = format!("{}:{}", nodes[j].host(), nodes[j].port());
            let _ = nodes[i].connect_to_peer(&peer_addr).await;
        }
    }
    Ok(())
}

/// Helper function to wait for message propagation
#[allow(dead_code)]
async fn wait_for_propagation(
    nodes: &[ChaincraftNode],
    expected_count: usize,
    timeout_secs: u64,
) -> bool {
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    while start.elapsed() < timeout {
        let counts: Vec<usize> = nodes.iter().map(|node| node.db_size()).collect();
        println!("Current message counts: {counts:?}");

        if counts.iter().all(|&count| count >= expected_count) {
            return true;
        }

        sleep(Duration::from_millis(500)).await;
    }
    false
}

#[tokio::test]
async fn test_node_creation_and_startup() -> Result<()> {
    // Create a node
    let mut node = ChaincraftNode::new_default();

    // Use an ephemeral port to avoid conflicts with other tests
    node.set_port(0);

    // Get the node ID and validate it's not empty
    let node_id = node.id().to_string();
    assert!(!node_id.is_empty(), "Node ID should not be empty");

    // Start the node
    node.start().await?;

    // Let it run for a brief moment
    sleep(Duration::from_millis(500)).await;

    // Verify that the node is running
    assert!(node.is_running(), "Node should be running after start");

    // Measure time to stop the node
    let start_time = Instant::now();
    node.close().await?;
    let stop_duration = start_time.elapsed();

    // Verify that the node stopped within a reasonable time
    assert!(stop_duration < Duration::from_secs(5), "Node should stop within 5 seconds");

    // Verify that the node is no longer running
    assert!(!node.is_running(), "Node should not be running after stop");

    Ok(())
}

#[tokio::test]
async fn test_network_creation() -> Result<()> {
    let num_nodes = 5;
    // Use a dedicated port range for this test
    let mut nodes = create_network_with_base_port(num_nodes, true, 8400).await?;

    // Start all nodes
    for node in &mut nodes {
        node.start().await?;
    }

    connect_nodes(&mut nodes).await?;

    // Wait for initial connections to establish
    sleep(Duration::from_secs(2)).await;

    // Test assertions
    assert_eq!(nodes.len(), num_nodes);
    for node in &nodes {
        assert!(node.is_running());
        let peer_count = node.get_peers().await.len();
        assert!(peer_count <= node.max_peers());
    }

    // Cleanup
    for mut node in nodes {
        node.close().await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_object_creation_and_propagation() -> Result<()> {
    // Use a dedicated port range to avoid clashes with other tests
    let mut nodes = create_network_with_base_port(3, true, 8500).await?;

    // Start all nodes
    for node in &mut nodes {
        node.start().await?;
    }

    connect_nodes(&mut nodes).await?;
    sleep(Duration::from_secs(2)).await;

    // Create a message from the first node
    let test_message = "Test message";
    let _hash = nodes[0]
        .create_shared_message(test_message.to_string())
        .await?;

    // Wait for propagation to all nodes (similar to Python wait_for_propagation)
    assert!(
        wait_for_propagation(&nodes, 1, 30).await,
        "message did not propagate to all nodes"
    );

    // Cleanup
    for mut node in nodes {
        node.close().await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_multiple_object_creation() -> Result<()> {
    // Use a dedicated port range to avoid clashes with other tests
    let mut nodes = create_network_with_base_port(3, true, 8600).await?;

    // Start all nodes
    for node in &mut nodes {
        node.start().await?;
    }

    connect_nodes(&mut nodes).await?;
    sleep(Duration::from_secs(2)).await;

    // Create multiple messages
    let node_count = nodes.len();
    for i in 0..3 {
        let message = format!("Object {i}");
        nodes[i % node_count].create_shared_message(message).await?;
        sleep(Duration::from_secs(1)).await;
    }

    // Expect all nodes to have 3 messages after propagation
    assert!(
        wait_for_propagation(&nodes, 3, 30).await,
        "messages did not propagate to all nodes"
    );

    // Cleanup
    for mut node in nodes {
        node.close().await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_network_resilience() -> Result<()> {
    // Use a dedicated port range to avoid clashes with other tests
    let mut nodes = create_network_with_base_port(4, true, 8700).await?;

    // Start all nodes
    for node in &mut nodes {
        node.start().await?;
    }

    connect_nodes(&mut nodes).await?;
    sleep(Duration::from_secs(2)).await;

    // Create initial message
    let initial_message = "Initial message";
    let initial_hash = nodes[0]
        .create_shared_message(initial_message.to_string())
        .await?;

    // Collect node addresses before any mutations
    let node_addrs: Vec<String> = nodes
        .iter()
        .map(|node| format!("{}:{}", node.host(), node.port()))
        .collect();

    // Simulate node failure by removing a node
    let mut failed_node = nodes.pop().unwrap();
    failed_node.close().await?;

    // Create new message with remaining nodes
    let new_message = "New message";
    let new_hash = nodes[0]
        .create_shared_message(new_message.to_string())
        .await?;

    // Restart failed node
    let mut restarted_node = ChaincraftNode::builder()
        .with_persistent_storage(false)
        .build()?;
    restarted_node.start().await?;

    // Reconnect to existing nodes
    for addr in &node_addrs[..node_addrs.len() - 1] {
        // Skip the last one (the failed node)
        let _ = restarted_node.connect_to_peer(addr).await;
    }

    nodes.push(restarted_node);

    // Verify basic functionality
    assert!(nodes[0].has_object(&initial_hash), "Initial message not found");
    assert!(nodes[0].has_object(&new_hash), "New message not found");

    // Cleanup
    for mut node in nodes {
        node.close().await?;
    }

    Ok(())
}
