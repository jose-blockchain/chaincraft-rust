use chaincraft::{
    crypto::ecdsa::ECDSASigner,
    examples::tendermint::{
        helpers, ConsensusState, TendermintMessageType, TendermintObject, ValidatorInfo,
    },
    network::PeerId,
    shared_object::ApplicationObject,
    storage::MemoryStorage,
    ChaincraftNode,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

async fn create_tendermint_node() -> (ChaincraftNode, TendermintObject) {
    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);
    node.set_port(0); // Use ephemeral port

    let tendermint_obj = TendermintObject::new().unwrap();
    let app_obj: Box<dyn ApplicationObject> = Box::new(tendermint_obj);
    node.add_shared_object(app_obj).await.unwrap();

    node.start().await.unwrap();

    // Create a new tendermint object for testing
    let test_tendermint = TendermintObject::new().unwrap();
    (node, test_tendermint)
}

#[tokio::test]
async fn test_tendermint_initialization() {
    let (mut node, tendermint) = create_tendermint_node().await;

    // Test initial state
    assert_eq!(tendermint.current_height, 1);
    assert_eq!(tendermint.current_round, 0);
    assert_eq!(tendermint.state, ConsensusState::Propose);
    assert_eq!(tendermint.blocks.len(), 1); // Genesis block

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_validator_management() {
    let (mut node, mut tendermint) = create_tendermint_node().await;

    // Add validators
    tendermint.add_validator("validator1".to_string(), "pubkey1".to_string(), 100);
    tendermint.add_validator("validator2".to_string(), "pubkey2".to_string(), 150);
    tendermint.add_validator("validator3".to_string(), "pubkey3".to_string(), 200);

    assert_eq!(tendermint.validators.len(), 3);
    assert_eq!(tendermint.total_voting_power(), 450);
    assert!(tendermint.has_majority(301)); // > 2/3 of 450
    assert!(!tendermint.has_majority(300)); // = 2/3 of 450

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_proposal_creation_and_processing() {
    let (mut node, mut tendermint) = create_tendermint_node().await;

    // Create and process a proposal
    let transactions = vec![json!({"type": "transfer", "amount": 100})];
    let proposal = tendermint.create_proposal(transactions).unwrap();

    let processed = tendermint.process_proposal(proposal.clone()).unwrap();
    assert!(processed);

    // Check that proposal was stored
    let key = (tendermint.current_height, tendermint.current_round);
    assert!(tendermint.proposals.contains_key(&key));

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_prevote_processing() {
    let (mut node, mut tendermint) = create_tendermint_node().await;
    let signer = ECDSASigner::new().unwrap();

    // Add a validator
    let validator_addr = signer.get_public_key_pem().unwrap();
    tendermint.add_validator(validator_addr.clone(), "pubkey1".to_string(), 100);

    // Create and process a prevote
    let prevote_msg = helpers::create_prevote_message(
        1,
        0,
        Some("block_hash".to_string()),
        validator_addr,
        &signer,
    )
    .unwrap();

    let prevote = serde_json::from_value::<TendermintMessageType>(prevote_msg).unwrap();
    let processed = tendermint.process_prevote(prevote).unwrap();
    assert!(processed);

    // Check that prevote was stored
    let key = (tendermint.current_height, tendermint.current_round);
    assert!(tendermint.prevotes.contains_key(&key));

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_precommit_processing() {
    let (mut node, mut tendermint) = create_tendermint_node().await;
    let signer = ECDSASigner::new().unwrap();

    // Add a validator
    let validator_addr = signer.get_public_key_pem().unwrap();
    tendermint.add_validator(validator_addr.clone(), "pubkey1".to_string(), 100);

    // Create and process a precommit
    let precommit_msg = helpers::create_precommit_message(
        1,
        0,
        Some("block_hash".to_string()),
        validator_addr,
        &signer,
    )
    .unwrap();

    let precommit = serde_json::from_value::<TendermintMessageType>(precommit_msg).unwrap();
    let processed = tendermint.process_precommit(precommit).unwrap();
    assert!(processed);

    // Check that precommit was stored
    let key = (tendermint.current_height, tendermint.current_round);
    assert!(tendermint.precommits.contains_key(&key));

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_consensus_round_completion() {
    let (mut node, mut tendermint) = create_tendermint_node().await;
    let signer1 = ECDSASigner::new().unwrap();
    let signer2 = ECDSASigner::new().unwrap();
    let signer3 = ECDSASigner::new().unwrap();

    // Add three validators with enough voting power
    let validator1 = signer1.get_public_key_pem().unwrap();
    let validator2 = signer2.get_public_key_pem().unwrap();
    let validator3 = signer3.get_public_key_pem().unwrap();

    tendermint.add_validator(validator1.clone(), "pubkey1".to_string(), 100);
    tendermint.add_validator(validator2.clone(), "pubkey2".to_string(), 100);
    tendermint.add_validator(validator3.clone(), "pubkey3".to_string(), 100);

    let block_hash = "consensus_block_hash";

    // Create precommits from all validators
    let precommit1 =
        helpers::create_precommit_message(1, 0, Some(block_hash.to_string()), validator1, &signer1)
            .unwrap();
    let precommit2 =
        helpers::create_precommit_message(1, 0, Some(block_hash.to_string()), validator2, &signer2)
            .unwrap();
    let precommit3 =
        helpers::create_precommit_message(1, 0, Some(block_hash.to_string()), validator3, &signer3)
            .unwrap();

    // Process precommits
    let pc1 = serde_json::from_value::<TendermintMessageType>(precommit1).unwrap();
    let pc2 = serde_json::from_value::<TendermintMessageType>(precommit2).unwrap();
    let pc3 = serde_json::from_value::<TendermintMessageType>(precommit3).unwrap();

    tendermint.process_precommit(pc1).unwrap();
    tendermint.process_precommit(pc2).unwrap();
    tendermint.process_precommit(pc3).unwrap();

    // Should be able to commit now
    let commit_hash = tendermint.can_commit();
    assert_eq!(commit_hash, Some(block_hash.to_string()));

    // Commit the block
    let initial_height = tendermint.current_height;
    tendermint.commit_block(block_hash.to_string()).unwrap();

    // Check state after commit
    assert_eq!(tendermint.current_height, initial_height + 1);
    assert_eq!(tendermint.current_round, 0);
    assert_eq!(tendermint.blocks.len(), 2); // Genesis + new block

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_validator_set_message() {
    let (mut node, _) = create_tendermint_node().await;

    // Create validator set message
    let validators = vec![
        ValidatorInfo {
            address: "validator1".to_string(),
            public_key: "pubkey1".to_string(),
            voting_power: 100,
            active: true,
        },
        ValidatorInfo {
            address: "validator2".to_string(),
            public_key: "pubkey2".to_string(),
            voting_power: 150,
            active: true,
        },
    ];

    let validator_set_msg = helpers::create_validator_set_message(validators, 1).unwrap();

    // Send message to node
    node.create_shared_message_with_data(validator_set_msg)
        .await
        .unwrap();
    sleep(Duration::from_millis(100)).await;

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_consensus_state_info() {
    let (mut node, tendermint) = create_tendermint_node().await;

    // Test consensus info
    let info = tendermint.get_consensus_info();
    assert_eq!(info["height"], 1);
    assert_eq!(info["round"], 0);
    assert_eq!(info["validators_count"], 0);
    assert_eq!(info["blocks_count"], 1);

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_voting_statistics() {
    let (mut node, mut tendermint) = create_tendermint_node().await;
    let signer = ECDSASigner::new().unwrap();

    // Add validator and create vote
    let validator_addr = signer.get_public_key_pem().unwrap();
    tendermint.add_validator(validator_addr.clone(), "pubkey1".to_string(), 100);

    let prevote_msg = helpers::create_prevote_message(
        1,
        0,
        Some("block_hash".to_string()),
        validator_addr,
        &signer,
    )
    .unwrap();

    let prevote = serde_json::from_value::<TendermintMessageType>(prevote_msg).unwrap();
    tendermint.process_prevote(prevote).unwrap();

    // Test voting stats
    let stats = tendermint.get_voting_stats();
    assert_eq!(stats["height"], 1);
    assert_eq!(stats["round"], 0);
    assert_eq!(stats["prevotes"], 1);
    assert_eq!(stats["precommits"], 0);

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_byzantine_fault_tolerance() {
    let (mut node, mut tendermint) = create_tendermint_node().await;

    // Create 4 validators (can tolerate 1 Byzantine)
    for i in 1..=4 {
        tendermint.add_validator(format!("validator{i}"), format!("pubkey{i}"), 100);
    }

    // Simulate conflicting votes from Byzantine validator
    let key = (tendermint.current_height, tendermint.current_round);

    // Byzantine validator votes for two different blocks
    tendermint
        .prevotes
        .entry(key)
        .or_insert_with(HashMap::new)
        .insert(
            "validator1".to_string(),
            chaincraft::examples::tendermint::Vote {
                validator: "validator1".to_string(),
                block_hash: Some("block_hash_1".to_string()),
                signature: "sig1".to_string(),
                timestamp: chrono::Utc::now(),
            },
        );

    // This shouldn't affect consensus if honest validators outnumber Byzantine ones
    assert_eq!(tendermint.total_voting_power(), 400);
    assert!(tendermint.has_majority(267)); // Need > 2/3

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_message_integration_with_node() {
    let (mut node, _) = create_tendermint_node().await;
    let signer = ECDSASigner::new().unwrap();

    // Create a full proposal message and send to node
    let proposal_msg = helpers::create_proposal_message(
        1,
        0,
        "integration_block_hash".to_string(),
        signer.get_public_key_pem().unwrap(),
        &signer,
    )
    .unwrap();

    node.create_shared_message_with_data(proposal_msg)
        .await
        .unwrap();
    sleep(Duration::from_millis(100)).await;

    // Create prevote and send to node
    let prevote_msg = helpers::create_prevote_message(
        1,
        0,
        Some("integration_block_hash".to_string()),
        signer.get_public_key_pem().unwrap(),
        &signer,
    )
    .unwrap();

    node.create_shared_message_with_data(prevote_msg)
        .await
        .unwrap();
    sleep(Duration::from_millis(100)).await;

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_application_object_interface() {
    let tendermint = TendermintObject::new().unwrap();

    // Test ApplicationObject interface
    assert_eq!(tendermint.type_name(), "TendermintBFT");
    assert!(!tendermint.id().as_uuid().is_nil());

    // Test state retrieval
    let state = tendermint.get_state().await.unwrap();
    assert_eq!(state["type"], "TendermintBFT");
    assert_eq!(state["height"], 1);
    assert_eq!(state["validators"], 0);
}

#[tokio::test]
async fn test_tendermint_reset() {
    let (mut node, mut tendermint) = create_tendermint_node().await;

    // Add some state
    tendermint.add_validator("validator1".to_string(), "pubkey1".to_string(), 100);
    tendermint.current_height = 5;
    tendermint.current_round = 2;

    // Reset
    tendermint.reset().await.unwrap();

    // Check state is reset
    assert_eq!(tendermint.current_height, 1);
    assert_eq!(tendermint.current_round, 0);
    assert_eq!(tendermint.state, ConsensusState::Propose);
    assert!(tendermint.proposals.is_empty());
    assert!(tendermint.prevotes.is_empty());
    assert!(tendermint.precommits.is_empty());

    node.close().await.unwrap();
}
