use chaincraft_rust::examples::chatroom::{helpers, ChatroomObject};
use chaincraft_rust::{
    clear_local_registry,
    crypto::ecdsa::{ECDSASigner, ECDSAVerifier},
    network::PeerId,
    shared_object::ApplicationObject,
    storage::MemoryStorage,
    ChaincraftNode,
};
use serde_json::json;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

async fn create_node_with_chatroom() -> ChaincraftNode {
    clear_local_registry();
    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);

    // Use ephemeral port to avoid conflicts
    node.set_port(0);
    node.disable_local_discovery();

    // Add a ChatroomObject to the node
    let chatroom_obj: Box<dyn ApplicationObject> = Box::new(ChatroomObject::new());
    node.add_shared_object(chatroom_obj).await.unwrap();

    node.start().await.unwrap();
    node
}

#[tokio::test]
async fn test_chatroom_creation() {
    let mut node = create_node_with_chatroom().await;

    // Create an ECDSA signer for admin
    let admin_signer = ECDSASigner::new().unwrap();

    // Create a chatroom
    let create_msg =
        helpers::create_chatroom_message("test_room".to_string(), &admin_signer).unwrap();

    // Send the message to the node
    node.create_shared_message_with_data(create_msg)
        .await
        .unwrap();

    sleep(Duration::from_millis(100)).await;

    // Verify the chatroom was created by checking shared objects
    let shared_objects = node.shared_objects().await;
    if let Some(obj) = shared_objects.first() {
        if let Some(chatroom_obj) = obj.as_any().downcast_ref::<ChatroomObject>() {
            let chatrooms = chatroom_obj.get_chatrooms();
            assert!(chatrooms.contains_key("test_room"));

            let room = &chatrooms["test_room"];
            assert_eq!(room.name, "test_room");
            assert_eq!(room.admin, admin_signer.get_public_key_pem().unwrap());
            assert!(room
                .members
                .contains(&admin_signer.get_public_key_pem().unwrap()));
        } else {
            panic!("Expected ChatroomObject");
        }
    } else {
        panic!("No shared objects found");
    }

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_chatroom_posting() {
    let mut node = create_node_with_chatroom().await;

    // Create an ECDSA signer for admin
    let admin_signer = ECDSASigner::new().unwrap();

    // Create a chatroom
    let create_msg =
        helpers::create_chatroom_message("chat_room".to_string(), &admin_signer).unwrap();

    node.create_shared_message_with_data(create_msg)
        .await
        .unwrap();
    sleep(Duration::from_millis(100)).await;

    // Post a message
    let post_msg = helpers::create_post_message(
        "chat_room".to_string(),
        "Hello, world!".to_string(),
        &admin_signer,
    )
    .unwrap();

    node.create_shared_message_with_data(post_msg)
        .await
        .unwrap();
    sleep(Duration::from_millis(100)).await;

    // Verify the message was posted
    let shared_objects = node.shared_objects().await;
    if let Some(obj) = shared_objects.first() {
        if let Some(chatroom_obj) = obj.as_any().downcast_ref::<ChatroomObject>() {
            let chatrooms = chatroom_obj.get_chatrooms();
            let room = &chatrooms["chat_room"];

            // Find POST_MESSAGE type messages
            let post_messages: Vec<_> = room
                .messages
                .iter()
                .filter(|m| m.message_type == "POST_MESSAGE")
                .collect();

            assert_eq!(post_messages.len(), 1);
            assert_eq!(post_messages[0].text.as_ref().unwrap(), "Hello, world!");
            assert_eq!(post_messages[0].public_key_pem, admin_signer.get_public_key_pem().unwrap());
        }
    }

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_signature_verification() {
    // Test that the ECDSA signatures work correctly
    let signer = ECDSASigner::new().unwrap();
    let verifier = ECDSAVerifier::new();

    let message = b"test message";
    let signature = signer.sign(message).unwrap();
    let public_key_pem = signer.get_public_key_pem().unwrap();

    // Verify the signature
    let is_valid = verifier
        .verify(message, &signature, &public_key_pem)
        .unwrap();
    assert!(is_valid);

    // Test with wrong message
    let wrong_message = b"wrong message";
    let is_invalid = verifier
        .verify(wrong_message, &signature, &public_key_pem)
        .unwrap();
    assert!(!is_invalid);
}

#[tokio::test]
async fn test_invalid_chatroom_message() {
    let mut node = create_node_with_chatroom().await;

    // Create an invalid message (missing required fields)
    let invalid_msg = json!({
        "message_type": "CREATE_CHATROOM",
        "chatroom_name": "test_room"
        // Missing public_key_pem, timestamp, signature
    });

    // This should not crash, but the message should be ignored
    let _result = node.create_shared_message_with_data(invalid_msg).await;

    sleep(Duration::from_millis(100)).await;

    // Verify no chatroom was created
    let shared_objects = node.shared_objects().await;
    if let Some(obj) = shared_objects.first() {
        if let Some(chatroom_obj) = obj.as_any().downcast_ref::<ChatroomObject>() {
            let chatrooms = chatroom_obj.get_chatrooms();
            assert!(chatrooms.is_empty());
        }
    }

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_chatroom_state() {
    let mut node = create_node_with_chatroom().await;

    // Create an ECDSA signer
    let signer = ECDSASigner::new().unwrap();

    // Create a chatroom
    let create_msg = helpers::create_chatroom_message("state_room".to_string(), &signer).unwrap();

    node.create_shared_message_with_data(create_msg)
        .await
        .unwrap();
    sleep(Duration::from_millis(100)).await;

    // Check the state
    let shared_objects = node.shared_objects().await;
    if let Some(obj) = shared_objects.first() {
        let state = obj.get_state().await.unwrap();
        assert_eq!(state["chatroom_count"], 1);
        assert_eq!(state["total_messages"], 0); // No chat messages yet, just system messages

        let chatrooms = state["chatrooms"].as_array().unwrap();
        assert_eq!(chatrooms.len(), 1);
        assert_eq!(chatrooms[0], "state_room");
    }

    node.close().await.unwrap();
}
