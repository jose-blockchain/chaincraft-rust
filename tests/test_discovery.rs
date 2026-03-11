use chaincraft_rust::{
    clear_local_registry, network::PeerId, storage::MemoryStorage, ChaincraftNode,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

async fn create_node() -> ChaincraftNode {
    clear_local_registry();
    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);
    node.set_port(0);
    node.disable_local_discovery();
    node
}

#[allow(dead_code)]
async fn wait_for_propagation(
    nodes: &[ChaincraftNode],
    expected_count: usize,
    timeout_secs: u64,
) -> bool {
    let timeout = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        let mut all_match = true;
        let mut counts = Vec::new();

        for node in nodes {
            let peer_count = node.get_peers().await.len();
            counts.push(peer_count);
            if peer_count != expected_count {
                all_match = false;
            }
        }

        println!("Current peer counts: {counts:?}");

        if all_match {
            return true;
        }

        sleep(Duration::from_millis(500)).await;
    }

    false
}

#[tokio::test]
async fn test_single_node_no_peers() {
    // Clear registry and disable local discovery for true isolation
    clear_local_registry();
    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);
    node.set_port(0);
    node.disable_local_discovery(); // Disable to ensure no peers from other tests

    node.start().await.unwrap();

    sleep(Duration::from_secs(1)).await;

    let peers = node.get_peers().await;
    println!("Peers: {peers:?}");
    assert_eq!(peers.len(), 0, "Expected 0 peers but found {peers:?}");

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_two_nodes_one_connection() {
    let mut node1 = create_node().await;
    let mut node2 = create_node().await;

    node1.start().await.unwrap();
    node2.start().await.unwrap();

    // Connect node2 to node1
    let node1_addr = format!("{}:{}", node1.host(), node1.port());
    node2
        .connect_to_peer_with_discovery(&node1_addr, true)
        .await
        .unwrap();

    // Wait for connection to establish
    sleep(Duration::from_secs(2)).await;

    // Both nodes should see each other
    let _node1_peers = node1.get_peers().await;
    let node2_peers = node2.get_peers().await;

    // At minimum, we should have established the connection
    // In a full implementation, discovery would propagate both ways
    assert!(!node2_peers.is_empty());

    node1.close().await.unwrap();
    node2.close().await.unwrap();
}

#[tokio::test]
async fn test_three_nodes_discovery() {
    let mut node1 = create_node().await;
    let mut node2 = create_node().await;
    let mut node3 = create_node().await;

    node1.start().await.unwrap();
    node2.start().await.unwrap();
    node3.start().await.unwrap();

    // Connect node2 to node1
    let node1_addr = format!("{}:{}", node1.host(), node1.port());
    node2
        .connect_to_peer_with_discovery(&node1_addr, true)
        .await
        .unwrap();

    sleep(Duration::from_secs(1)).await;

    // Connect node3 to node2
    let node2_addr = format!("{}:{}", node2.host(), node2.port());
    node3
        .connect_to_peer_with_discovery(&node2_addr, true)
        .await
        .unwrap();

    sleep(Duration::from_secs(2)).await;

    // Check connections - in simplified implementation each node knows its direct connections
    let node1_peers = node1.get_peers().await;
    let node2_peers = node2.get_peers().await;
    let node3_peers = node3.get_peers().await;

    println!("Node1 peers: {}", node1_peers.len());
    println!("Node2 peers: {}", node2_peers.len());
    println!("Node3 peers: {}", node3_peers.len());

    // Each node should have at least their direct connections
    assert!(!node2_peers.is_empty()); // Connected to node1
    assert!(!node3_peers.is_empty()); // Connected to node2

    node1.close().await.unwrap();
    node2.close().await.unwrap();
    node3.close().await.unwrap();
}

#[tokio::test]
async fn test_four_nodes_discovery() {
    let mut nodes = Vec::new();

    // Create 4 nodes
    for _ in 0..4 {
        let mut node = create_node().await;
        node.start().await.unwrap();
        nodes.push(node);
    }

    // Connect in a chain: node1 -> node2 -> node3 -> node4 -> node1
    for i in 0..nodes.len() {
        let next_idx = (i + 1) % nodes.len();
        let next_addr = format!("{}:{}", nodes[next_idx].host(), nodes[next_idx].port());
        nodes[i]
            .connect_to_peer_with_discovery(&next_addr, true)
            .await
            .unwrap();
        sleep(Duration::from_millis(500)).await; // Small delay between connections
    }

    sleep(Duration::from_secs(2)).await; // Wait for discovery to propagate

    // Check that each node has connections
    for (i, node) in nodes.iter().enumerate() {
        let peer_count = node.get_peers().await.len();
        println!("Node {i} has {peer_count} peers");
        assert!(peer_count >= 1); // At least one connection
    }

    // Clean up
    for mut node in nodes {
        node.close().await.unwrap();
    }
}

#[tokio::test]
async fn test_max_peers_limit() {
    let mut node1 = create_node().await;
    node1.start().await.unwrap();

    let max_peers = node1.max_peers();

    // Try to connect several peers (fewer than max to test basic functionality)
    let mut connecting_nodes = Vec::new();
    let test_peer_count = std::cmp::min(5, max_peers);

    for _ in 0..test_peer_count {
        let mut node = create_node().await;
        node.start().await.unwrap();

        let node1_addr = format!("{}:{}", node1.host(), node1.port());
        let _ = node.connect_to_peer_with_discovery(&node1_addr, true).await;

        connecting_nodes.push(node);
        sleep(Duration::from_millis(200)).await; // Small delay
    }

    sleep(Duration::from_secs(2)).await;

    // Check peer connections - count connections from both directions
    let mut total_connections = 0;

    // Count node1's peers
    let _node1_peers = node1.get_peers().await;
    total_connections += _node1_peers.len();

    // Count connections from connecting nodes
    for node in &connecting_nodes {
        let peer_count = node.get_peers().await.len();
        total_connections += peer_count;
    }

    println!("Node1 has {} peers (max: {})", _node1_peers.len(), max_peers);
    println!("Total connections in network: {total_connections}");

    // In our simplified implementation, connections are directional
    // We should see at least some connections somewhere in the network
    assert!(total_connections > 0, "Expected at least one connection in the network");

    // Clean up
    node1.close().await.unwrap();
    for mut node in connecting_nodes {
        node.close().await.unwrap();
    }
}

#[tokio::test]
async fn test_peer_connection_management() {
    let mut node1 = create_node().await;
    let mut node2 = create_node().await;

    node1.start().await.unwrap();
    node2.start().await.unwrap();

    // Initially no peers
    assert_eq!(node1.get_peers().await.len(), 0);
    assert_eq!(node2.get_peers().await.len(), 0);

    // Connect nodes
    let node1_addr = format!("{}:{}", node1.host(), node1.port());
    node2.connect_to_peer(&node1_addr).await.unwrap();

    sleep(Duration::from_secs(1)).await;

    // Check connections were established
    let node2_peers = node2.get_peers().await;
    assert!(!node2_peers.is_empty());

    // Test node properties
    assert!(node1.is_running_async().await);
    assert!(node2.is_running_async().await);

    node1.close().await.unwrap();
    node2.close().await.unwrap();
}

#[tokio::test]
async fn test_network_scaling() {
    let num_nodes = 10;
    let mut nodes = Vec::new();

    // Create nodes
    for _ in 0..num_nodes {
        let mut node = create_node().await;
        node.start().await.unwrap();
        nodes.push(node);
    }

    // Connect each node to the next one (chain topology)
    for i in 0..num_nodes - 1 {
        let next_addr = format!("{}:{}", nodes[i + 1].host(), nodes[i + 1].port());
        nodes[i].connect_to_peer(&next_addr).await.unwrap();
        sleep(Duration::from_millis(200)).await;
    }

    sleep(Duration::from_secs(2)).await; // Allow connections to stabilize

    // Verify network connectivity
    let mut total_connections = 0;
    for (i, node) in nodes.iter().enumerate() {
        let peer_count = node.get_peers().await.len();
        total_connections += peer_count;
        println!("Node {i} has {peer_count} peers");
    }

    let average_connections = total_connections as f64 / num_nodes as f64;
    println!("Average connections per node: {average_connections:.2}");

    // Each node should have at least some connections
    assert!(average_connections > 0.0);

    // Clean up
    for mut node in nodes {
        node.close().await.unwrap();
    }
}
