//! Tests for persisted peers and banned peers (PEERS / BANNED_PEERS in DB)

use chaincraft::{clear_local_registry, network::PeerId, storage::MemoryStorage, ChaincraftNode};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::test]
async fn test_ban_peer_rejects_connection() {
    clear_local_registry();
    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);
    node.set_port(0);
    node.disable_local_discovery();
    node.start().await.unwrap();

    let banned_addr: SocketAddr = "127.0.0.1:19999".parse().unwrap();
    node.ban_peer(banned_addr, None).await.unwrap();

    let result = node.connect_to_peer("127.0.0.1:19999").await;
    assert!(result.is_err());

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_unban_peer_allows_connection() {
    clear_local_registry();
    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);
    node.set_port(0);
    node.disable_local_discovery();
    node.start().await.unwrap();

    let addr: SocketAddr = "127.0.0.1:29999".parse().unwrap();
    node.ban_peer(addr, None).await.unwrap();
    assert!(node.is_banned(addr).await);

    node.unban_peer(addr).await.unwrap();
    assert!(!node.is_banned(addr).await);

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_persisted_peers_reload() {
    clear_local_registry();
    let storage = Arc::new(MemoryStorage::new());
    let id = PeerId::new();
    let mut node1 = ChaincraftNode::new(id, storage.clone());
    node1.set_port(0);
    node1.disable_local_discovery();
    node1.start().await.unwrap();

    let id2 = PeerId::new();
    let mut node2 = ChaincraftNode::new(id2, storage.clone());
    node2.set_port(0);
    node2.disable_local_discovery();
    node2.start().await.unwrap();

    node1
        .connect_to_peer(&format!("127.0.0.1:{}", node2.port()))
        .await
        .unwrap();
    let peers_before = node1.get_peers().await;
    assert!(!peers_before.is_empty());
    node1.close().await.unwrap();
    node2.close().await.unwrap();

    let mut node3 = ChaincraftNode::new(PeerId::new(), storage);
    node3.set_port(0);
    node3.disable_local_discovery();
    node3.start().await.unwrap();

    let peers_after = node3.get_peers().await;
    assert!(!peers_after.is_empty(), "Persisted peers should reload on start");
    node3.close().await.unwrap();
}

#[tokio::test]
async fn test_banned_peers_persisted() {
    clear_local_registry();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(PeerId::new(), storage.clone());
    node.set_port(0);
    node.disable_local_discovery();
    node.start().await.unwrap();

    let addr: SocketAddr = "127.0.0.1:39999".parse().unwrap();
    node.ban_peer(addr, None).await.unwrap();
    node.close().await.unwrap();

    let mut node2 = ChaincraftNode::new(PeerId::new(), storage);
    node2.set_port(0);
    node2.disable_local_discovery();
    node2.start().await.unwrap();

    assert!(node2.is_banned(addr).await, "Banned peer should persist across restarts");
    node2.close().await.unwrap();
}
