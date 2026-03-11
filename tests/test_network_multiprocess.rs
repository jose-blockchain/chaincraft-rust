use std::process::Command;
use std::time::Duration;
use std::{env, fs};

use serde_json::Value;
use tempfile::tempdir;

/// End-to-end multi-process network test.
///
/// Spawns three separate `network_worker` processes, connects them in a small
/// graph, sends a message from node1, and verifies that all nodes see at least
/// one stored message in their local storage.
#[test]
fn test_multiprocess_message_propagation() {
    // Path to the compiled network_worker binary
    let worker_bin = env!("CARGO_BIN_EXE_network_worker");

    let temp = tempdir().expect("failed to create tempdir");
    let out1 = temp.path().join("node1.json");
    let out2 = temp.path().join("node2.json");
    let out3 = temp.path().join("node3.json");

    // Fixed ports for this test
    let port1 = 9101u16;
    let port2 = 9102u16;
    let port3 = 9103u16;

    // Spawn node2 and node3 first (listeners)
    let mut child2 = Command::new(worker_bin)
        .args([
            "--port",
            &port2.to_string(),
            "--peer",
            &format!("127.0.0.1:{port1}"),
            "--output",
            out2.to_str().unwrap(),
        ])
        .spawn()
        .expect("failed to spawn node2");

    let mut child3 = Command::new(worker_bin)
        .args([
            "--port",
            &port3.to_string(),
            "--peer",
            &format!("127.0.0.1:{port1}"),
            "--output",
            out3.to_str().unwrap(),
        ])
        .spawn()
        .expect("failed to spawn node3");

    // Small delay to give node2/3 time to bind sockets
    std::thread::sleep(Duration::from_millis(500));

    // Spawn node1, which will send a message and connect to node2 and node3
    let mut child1 = Command::new(worker_bin)
        .args([
            "--port",
            &port1.to_string(),
            "--peer",
            &format!("127.0.0.1:{port2}"),
            "--peer",
            &format!("127.0.0.1:{port3}"),
            "--message",
            "hello_multiprocess",
            "--output",
            out1.to_str().unwrap(),
        ])
        .spawn()
        .expect("failed to spawn node1");

    // Wait for all workers to finish
    assert!(child1.wait().expect("child1 failed").success());
    assert!(child2.wait().expect("child2 failed").success());
    assert!(child3.wait().expect("child3 failed").success());

    // Read outputs
    let data1: Value =
        serde_json::from_str(&fs::read_to_string(&out1).expect("failed to read node1 output"))
            .expect("invalid JSON from node1");
    let data2: Value =
        serde_json::from_str(&fs::read_to_string(&out2).expect("failed to read node2 output"))
            .expect("invalid JSON from node2");
    let data3: Value =
        serde_json::from_str(&fs::read_to_string(&out3).expect("failed to read node3 output"))
            .expect("invalid JSON from node3");

    let db1 = data1["db_size"].as_u64().unwrap_or(0);
    let db2 = data2["db_size"].as_u64().unwrap_or(0);
    let db3 = data3["db_size"].as_u64().unwrap_or(0);

    // At minimum, node1 should have stored its own message, and the others
    // should have received and stored it via UDP propagation.
    assert!(db1 >= 1, "expected node1 to have at least 1 message, got {db1}");
    assert!(db2 >= 1, "expected node2 to have at least 1 message, got {db2}");
    assert!(db3 >= 1, "expected node3 to have at least 1 message, got {db3}");
}
