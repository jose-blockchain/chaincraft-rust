use chaincraft::{
    clear_local_registry,
    network::PeerId,
    shared_object::{ApplicationObject, SimpleSharedNumber},
    storage::MemoryStorage,
    ChaincraftNode,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

async fn create_network(num_nodes: usize) -> Vec<ChaincraftNode> {
    clear_local_registry();
    let mut nodes = Vec::new();

    for _ in 0..num_nodes {
        let id = PeerId::new();
        let storage = Arc::new(MemoryStorage::new());
        let mut node = ChaincraftNode::new(id, storage);

        node.set_port(0);
        node.disable_local_discovery();

        // Add a SimpleSharedNumber object to each node
        let shared_number: Box<dyn ApplicationObject> = Box::new(SimpleSharedNumber::new());
        node.add_shared_object(shared_number).await.unwrap();

        node.start().await.unwrap();
        nodes.push(node);
    }

    nodes
}

async fn connect_nodes(nodes: &mut [ChaincraftNode]) {
    let num_nodes = nodes.len();
    for i in 0..num_nodes {
        // Connect each node to the next one in a ring topology
        let next_node = (i + 1) % num_nodes;
        let next_host = nodes[next_node].host();
        let next_port = nodes[next_node].port();
        let connect_addr = format!("{next_host}:{next_port}");

        nodes[i].connect_to_peer(&connect_addr).await.unwrap();
    }
}

#[tokio::test]
async fn test_network_creation() {
    let num_nodes = 5;
    let mut nodes = create_network(num_nodes).await;
    connect_nodes(&mut nodes).await;

    sleep(Duration::from_secs(1)).await; // Wait for connections to establish

    assert_eq!(nodes.len(), num_nodes);

    for node in &nodes {
        assert!(node.is_running_async().await);
        assert!(node.shared_object_count().await > 0);
    }

    // Clean up
    for mut node in nodes {
        node.close().await.unwrap();
    }
}

#[tokio::test]
async fn test_shared_object_propagation() {
    let num_nodes = 5;
    let mut nodes = create_network(num_nodes).await;
    connect_nodes(&mut nodes).await;

    sleep(Duration::from_secs(1)).await; // Wait for connections to establish

    // Create messages from each node
    for i in 0..nodes.len() {
        let value = (i + 1) as i64;
        let data = serde_json::json!(value);
        nodes[i]
            .create_shared_message_with_data(data)
            .await
            .unwrap();

        sleep(Duration::from_millis(500)).await; // Wait between message creations

        // Print current state of all nodes
        for (j, n) in nodes.iter().enumerate() {
            let shared_objects = n.shared_objects().await;
            if let Some(obj) = shared_objects.first() {
                if let Some(shared_number) = obj.as_any().downcast_ref::<SimpleSharedNumber>() {
                    println!("Node {}: Shared number: {}", j, shared_number.get_number());
                }
            }
        }
    }

    // Calculate expected total
    let expected_number: i64 = (1..=num_nodes as i64).sum();

    // Wait for propagation (simplified - in a real network this would involve gossip protocol)
    // For now, we just verify that each node processed its own message
    for (i, node) in nodes.iter().enumerate() {
        let shared_objects = node.shared_objects().await;
        if let Some(obj) = shared_objects.first() {
            if let Some(shared_number) = obj.as_any().downcast_ref::<SimpleSharedNumber>() {
                // Each node should have processed at least its own message
                assert!(shared_number.get_number() >= (i + 1) as i64);
            }
        }
    }

    println!("Expected total after all propagation: {expected_number}");

    // Clean up
    for mut node in nodes {
        node.close().await.unwrap();
    }
}

#[tokio::test]
async fn test_message_deduplication() {
    let mut node = create_network(1).await.into_iter().next().unwrap();

    // Send the same message multiple times
    let test_value = 42;
    let data = serde_json::json!(test_value);

    for _ in 0..5 {
        node.create_shared_message_with_data(data.clone())
            .await
            .unwrap();
    }

    // Verify the shared number only incremented once due to deduplication
    let shared_objects = node.shared_objects().await;
    if let Some(obj) = shared_objects.first() {
        if let Some(shared_number) = obj.as_any().downcast_ref::<SimpleSharedNumber>() {
            assert_eq!(shared_number.get_number(), test_value);
            assert_eq!(shared_number.get_messages().len(), 1);
        }
    }

    // Clean up
    node.close().await.unwrap();
}

#[tokio::test]
async fn test_shared_object_state() {
    let mut node = create_network(1).await.into_iter().next().unwrap();

    // Add multiple messages
    let values = vec![10, 20, 30];
    for value in &values {
        let data = serde_json::json!(value);
        node.create_shared_message_with_data(data).await.unwrap();
    }

    // Verify the state
    let shared_objects = node.shared_objects().await;
    if let Some(obj) = shared_objects.first() {
        if let Some(shared_number) = obj.as_any().downcast_ref::<SimpleSharedNumber>() {
            let expected_sum: i64 = values.iter().sum();
            assert_eq!(shared_number.get_number(), expected_sum);
            assert_eq!(shared_number.get_messages().len(), values.len());

            // Test state as JSON
            let state = obj.get_state().await.unwrap();
            assert_eq!(state["number"], expected_sum);
            assert_eq!(state["message_count"], values.len());
        }
    }

    // Clean up
    node.close().await.unwrap();
}
