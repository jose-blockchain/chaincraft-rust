use chaincraft_rust::{network::PeerId, storage::MemoryStorage, ChaincraftNode};
use serde_json::json;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

async fn create_indexed_node() -> ChaincraftNode {
    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);
    // Use ephemeral port for single-node indexing tests (no peer connections needed)
    node.set_port(0);
    node.start().await.unwrap();
    node
}

async fn create_indexed_network(num_nodes: usize, base_port: u16) -> Vec<ChaincraftNode> {
    let mut nodes = Vec::new();
    for i in 0..num_nodes {
        let id = PeerId::new();
        let storage = Arc::new(MemoryStorage::new());
        let mut node = ChaincraftNode::new(id, storage);
        let port = base_port + i as u16;
        node.set_port(port);
        node.start().await.unwrap();
        nodes.push(node);
    }
    nodes
}

async fn connect_full_mesh(nodes: &mut [ChaincraftNode]) {
    // Simple full-mesh connectivity to exercise multi-node indexing scenario
    let len = nodes.len();
    for i in 0..len {
        for j in 0..len {
            if i == j {
                continue;
            }
            let addr = format!("{}:{}", nodes[j].host(), nodes[j].port());
            let _ = nodes[i].connect_to_peer(&addr).await;
        }
    }
}

#[tokio::test]
async fn test_basic_indexing_setup() {
    let mut node = create_indexed_node().await;

    // Test that the node starts properly with indexing
    assert!(node.is_running());

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_index_user_messages() {
    let mut node = create_indexed_node().await;

    // Create a user message
    let user_message = json!({
        "message_type": "User",
        "user_id": 1,
        "username": "alice",
        "email": "alice@example.com",
        "bio": "Hello, I'm Alice!"
    });

    node.create_shared_message_with_data(user_message)
        .await
        .unwrap();
    sleep(Duration::from_millis(100)).await;

    // Test passes if no errors occur during message creation
    // assert!(true);

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_index_post_messages() {
    let mut node = create_indexed_node().await;

    // Create a post message with tags
    let post_message = json!({
        "message_type": "Post",
        "post_id": 1,
        "title": "My First Post",
        "content": "Hello, world!",
        "tags": ["introduction", "greeting"],
        "likes": [1, 2, 3]
    });

    node.create_shared_message_with_data(post_message)
        .await
        .unwrap();
    sleep(Duration::from_millis(100)).await;

    // Test passes if no errors occur during message creation
    // assert!(true);

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_query_messages_by_type() {
    let mut node = create_indexed_node().await;

    // Create different message types
    let user_msg = json!({"message_type": "User", "username": "alice"});
    let post_msg = json!({"message_type": "Post", "title": "Hello World"});
    let comment_msg = json!({"message_type": "Comment", "text": "Nice post!"});

    node.create_shared_message_with_data(user_msg)
        .await
        .unwrap();
    node.create_shared_message_with_data(post_msg)
        .await
        .unwrap();
    node.create_shared_message_with_data(comment_msg)
        .await
        .unwrap();

    sleep(Duration::from_millis(100)).await;

    // Test passes if all messages are created without errors
    // assert!(true);

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_complex_message_indexing() {
    let mut node = create_indexed_node().await;

    // Create a complex nested message
    let complex_msg = json!({
        "message_type": "ComplexData",
        "metadata": {
            "created_at": "2024-01-01T00:00:00Z",
            "author": "test_user",
            "version": 1
        },
        "data": {
            "items": [
                {"id": 1, "name": "Item 1"},
                {"id": 2, "name": "Item 2"}
            ],
            "total_count": 2
        }
    });

    node.create_shared_message_with_data(complex_msg)
        .await
        .unwrap();
    sleep(Duration::from_millis(100)).await;

    // Test passes if complex message is created without errors
    // assert!(true);

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_timestamp_indexing() {
    let mut node = create_indexed_node().await;

    // Create messages with specific timestamps
    let msg1 = json!({"message_type": "Event", "event": "start"});
    let msg2 = json!({"message_type": "Event", "event": "middle"});
    let msg3 = json!({"message_type": "Event", "event": "end"});

    node.create_shared_message_with_data(msg1).await.unwrap();
    sleep(Duration::from_millis(10)).await;

    node.create_shared_message_with_data(msg2).await.unwrap();
    sleep(Duration::from_millis(10)).await;

    node.create_shared_message_with_data(msg3).await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // Test passes if all timed messages are created without errors
    // assert!(true);

    node.close().await.unwrap();
}

/// Multi-node indexing test similar in spirit to the Python indexing + network tests.
#[tokio::test]
async fn test_indexing_across_nodes() {
    // Use a dedicated base port range for this test
    let mut nodes = create_indexed_network(3, 8800).await;
    connect_full_mesh(&mut nodes).await;

    // Create a few messages on node 0 that would be indexable types in a full indexing backend
    let user_msg = json!({"message_type": "User", "username": "multi_node"});
    let post_msg = json!({"message_type": "Post", "title": "Indexed across nodes"});

    nodes[0]
        .create_shared_message_with_data(user_msg.clone())
        .await
        .unwrap();
    nodes[0]
        .create_shared_message_with_data(post_msg.clone())
        .await
        .unwrap();

    // Allow time for UDP propagation
    sleep(Duration::from_secs(2)).await;

    // All nodes should have at least 2 messages stored
    for (i, node) in nodes.iter().enumerate() {
        let count = node.db_size();
        assert!(
            count >= 2,
            "expected node {} to see at least 2 messages, got {}",
            i,
            count
        );
    }

    // Clean up
    for mut node in nodes {
        node.close().await.unwrap();
    }
}
