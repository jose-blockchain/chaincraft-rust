use anyhow::Result;
use chaincraft_rust::{
    crypto::ecdsa::{ECDSASigner, ECDSAVerifier},
    examples::randomness_beacon::{
        helpers, BeaconMessageType, BeaconValidator, RandomnessBeaconObject,
    },
    network::PeerId,
    shared_object::ApplicationObject,
    storage::MemoryStorage,
    ChaincraftNode,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

async fn create_beacon_node() -> (ChaincraftNode, RandomnessBeaconObject) {
    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);
    node.set_port(0); // Use ephemeral port

    let beacon_obj = RandomnessBeaconObject::new(60, 3).unwrap(); // 60 sec rounds, threshold 3
    let app_obj: Box<dyn ApplicationObject> = Box::new(beacon_obj);
    node.add_shared_object(app_obj).await.unwrap();

    node.start().await.unwrap();

    // Create a new beacon object for testing
    let test_beacon = RandomnessBeaconObject::new(60, 3).unwrap();
    (node, test_beacon)
}

#[tokio::test]
async fn test_beacon_initialization() {
    let (mut node, beacon) = create_beacon_node().await;

    // Test initial state
    assert_eq!(beacon.current_round, 1);
    assert_eq!(beacon.threshold, 3);
    assert_eq!(beacon.round_duration_secs, 60);
    assert!(beacon.validators.is_empty());
    assert!(beacon.rounds.is_empty());
    assert!(beacon.bias_resistance_enabled);

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_validator_registration() {
    let (mut node, mut beacon) = create_beacon_node().await;

    // Register validators
    let validator1 = BeaconValidator {
        address: "validator1".to_string(),
        public_key: "pubkey1".to_string(),
        vrf_key: "vrfkey1".to_string(),
        stake: 1000,
        active: true,
        last_participation: None,
    };

    let validator2 = BeaconValidator {
        address: "validator2".to_string(),
        public_key: "pubkey2".to_string(),
        vrf_key: "vrfkey2".to_string(),
        stake: 1500,
        active: true,
        last_participation: None,
    };

    beacon.register_validator(validator1).unwrap();
    beacon.register_validator(validator2).unwrap();

    assert_eq!(beacon.validators.len(), 2);
    assert!(beacon.validators.contains_key("validator1"));
    assert!(beacon.validators.contains_key("validator2"));

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_vrf_proof_generation() {
    let (mut node, beacon) = create_beacon_node().await;

    // Generate VRF proof
    let input = "random_input_123";
    let vrf_proof = beacon.generate_vrf_proof(input).unwrap();

    assert_eq!(vrf_proof.input, input);
    assert_eq!(vrf_proof.validator, beacon.my_validator_address);
    assert!(!vrf_proof.proof.is_empty());
    assert!(!vrf_proof.output.is_empty());
    assert!(!vrf_proof.signature.is_empty());

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_vrf_proof_processing() {
    let (mut node, mut beacon) = create_beacon_node().await;
    let _signer = ECDSASigner::new().unwrap();

    // Register a validator
    let validator_addr = _signer.get_public_key_pem().unwrap();
    let validator = BeaconValidator {
        address: validator_addr.clone(),
        public_key: "pubkey1".to_string(),
        vrf_key: "vrfkey1".to_string(),
        stake: 1000,
        active: true,
        last_participation: None,
    };
    beacon.register_validator(validator).unwrap();

    // Create VRF proof message
    let vrf_msg = BeaconMessageType::VrfProof {
        round: 1,
        input: "test_input".to_string(),
        proof: "test_proof".to_string(),
        output: "test_output".to_string(),
        validator: validator_addr,
        signature: "test_signature".to_string(),
        timestamp: chrono::Utc::now(),
    };

    let processed = beacon.process_vrf_proof(vrf_msg).unwrap();
    assert!(processed);

    // Check that VRF proof was stored
    assert!(beacon.pending_vrf_proofs.contains_key(&1));
    assert_eq!(beacon.pending_vrf_proofs[&1].len(), 1);

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_partial_signature_processing() {
    let (mut node, mut beacon) = create_beacon_node().await;
    let _signer = ECDSASigner::new().unwrap();

    // Register a validator
    let validator_addr = _signer.get_public_key_pem().unwrap();
    let validator = BeaconValidator {
        address: validator_addr.clone(),
        public_key: "pubkey1".to_string(),
        vrf_key: "vrfkey1".to_string(),
        stake: 1000,
        active: true,
        last_participation: None,
    };
    beacon.register_validator(validator).unwrap();

    // Create partial signature message
    let partial_sig_msg = BeaconMessageType::PartialSignature {
        round: 1,
        validator: validator_addr,
        partial_sig: "partial_signature_123".to_string(),
        signature: "signature_123".to_string(),
        timestamp: chrono::Utc::now(),
    };

    let processed = beacon.process_partial_signature(partial_sig_msg).unwrap();
    assert!(processed);

    // Check that partial signature was stored
    assert!(beacon.pending_partial_sigs.contains_key(&1));
    assert_eq!(beacon.pending_partial_sigs[&1].len(), 1);

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_round_finalization() {
    let (mut node, mut beacon) = create_beacon_node().await;

    // Register enough validators to meet threshold
    for i in 1..=4 {
        let validator = BeaconValidator {
            address: format!("validator{i}"),
            public_key: format!("pubkey{i}"),
            vrf_key: format!("vrfkey{i}"),
            stake: 1000,
            active: true,
            last_participation: None,
        };
        beacon.register_validator(validator).unwrap();
    }

    // Add enough VRF proofs
    for i in 1..=3 {
        let vrf_msg = BeaconMessageType::VrfProof {
            round: 1,
            input: "test_input".to_string(),
            proof: format!("proof_{i}"),
            output: format!("output_{i}"),
            validator: format!("validator{i}"),
            signature: format!("sig_{i}"),
            timestamp: chrono::Utc::now(),
        };
        beacon.process_vrf_proof(vrf_msg).unwrap();
    }

    // Add enough partial signatures
    for i in 1..=3 {
        let partial_sig_msg = BeaconMessageType::PartialSignature {
            round: 1,
            validator: format!("validator{i}"),
            partial_sig: format!("partial_sig_{i}"),
            signature: format!("sig_{i}"),
            timestamp: chrono::Utc::now(),
        };
        beacon.process_partial_signature(partial_sig_msg).unwrap();
    }

    // Should be able to finalize now
    assert!(beacon.can_finalize_round());

    let randomness = beacon.finalize_round().unwrap();
    assert!(!randomness.is_empty());
    assert_eq!(beacon.current_round, 2); // Advanced to next round
    assert_eq!(beacon.rounds.len(), 1); // One finalized round

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_bias_challenge_processing() {
    let (mut node, mut beacon) = create_beacon_node().await;

    // Create bias challenge
    let challenge_msg = BeaconMessageType::BiasChallenge {
        round: 1,
        challenger: "challenger1".to_string(),
        target_validator: "validator1".to_string(),
        challenge_data: "bias_evidence".to_string(),
        signature: "challenge_sig".to_string(),
    };

    let processed = beacon.process_bias_challenge(challenge_msg).unwrap();
    assert!(processed);

    // Check that challenge was stored
    assert!(beacon.challenges.contains_key(&1));
    assert_eq!(beacon.challenges[&1].len(), 1);

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_beacon_statistics() {
    let (mut node, mut beacon) = create_beacon_node().await;

    // Add some state
    for i in 1..=3 {
        let validator = BeaconValidator {
            address: format!("validator{i}"),
            public_key: format!("pubkey{i}"),
            vrf_key: format!("vrfkey{i}"),
            stake: 1000,
            active: true,
            last_participation: None,
        };
        beacon.register_validator(validator).unwrap();
    }

    let stats = beacon.get_beacon_stats();
    assert_eq!(stats["current_round"], 1);
    assert_eq!(stats["total_rounds"], 0);
    assert_eq!(stats["active_validators"], 3);
    assert_eq!(stats["threshold"], 3);
    assert_eq!(stats["current_vrf_proofs"], 0);
    assert_eq!(stats["current_partial_signatures"], 0);
    assert_eq!(stats["can_finalize"], false);
    assert_eq!(stats["bias_resistance"], true);

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_randomness_history() {
    let (mut node, mut beacon) = create_beacon_node().await;

    // Simulate a completed round
    beacon.current_round = 2;
    beacon.rounds.insert(
        1,
        chaincraft_rust::examples::randomness_beacon::BeaconRound {
            round: 1,
            randomness: "abc123def456".to_string(),
            participants: vec!["validator1".to_string(), "validator2".to_string()],
            vrf_proofs: vec![],
            threshold_signature: "threshold_sig".to_string(),
            finalized_at: chrono::Utc::now(),
            bias_challenges: vec![],
        },
    );

    let latest = beacon.get_latest_randomness();
    assert_eq!(latest, Some("abc123def456".to_string()));

    let history = beacon.get_randomness_history(5);
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].0, 1);
    assert_eq!(history[0].1, "abc123def456");

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_message_integration_with_node() {
    let (mut node, _beacon) = create_beacon_node().await;
    let _signer = ECDSASigner::new().unwrap();

    // Create validator registration message and send to node
    let reg_msg = BeaconMessageType::ValidatorRegistration {
        validator: "validator1".to_string(),
        public_key: "pubkey1".to_string(),
        vrf_key: "vrfkey1".to_string(),
        stake: 1000,
        signature: "reg_sig".to_string(),
    };

    let reg_data = serde_json::to_value(reg_msg).unwrap();
    node.create_shared_message_with_data(reg_data)
        .await
        .unwrap();
    sleep(Duration::from_millis(100)).await;

    // Create VRF proof message and send to node
    let vrf_msg = BeaconMessageType::VrfProof {
        round: 1,
        input: "integration_input".to_string(),
        proof: "integration_proof".to_string(),
        output: "integration_output".to_string(),
        validator: "validator1".to_string(),
        signature: "vrf_sig".to_string(),
        timestamp: chrono::Utc::now(),
    };

    let vrf_data = serde_json::to_value(vrf_msg).unwrap();
    node.create_shared_message_with_data(vrf_data)
        .await
        .unwrap();
    sleep(Duration::from_millis(100)).await;

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_application_object_interface() {
    let beacon = RandomnessBeaconObject::new(60, 3).unwrap();

    // Test ApplicationObject interface
    assert_eq!(beacon.type_name(), "RandomnessBeacon");
    assert!(!beacon.id().as_uuid().is_nil());

    // Test state retrieval
    let state = beacon.get_state().await.unwrap();
    assert_eq!(state["type"], "RandomnessBeacon");
    assert_eq!(state["current_round"], 1);
    assert_eq!(state["validators"], 0);
}

#[tokio::test]
async fn test_beacon_reset() {
    let (mut node, mut beacon) = create_beacon_node().await;

    // Add some state
    beacon.current_round = 5;
    for i in 1..=3 {
        let validator = BeaconValidator {
            address: format!("validator{i}"),
            public_key: format!("pubkey{i}"),
            vrf_key: format!("vrfkey{i}"),
            stake: 1000,
            active: true,
            last_participation: None,
        };
        beacon.register_validator(validator).unwrap();
    }

    // Reset
    beacon.reset().await.unwrap();

    // Check state is reset
    assert_eq!(beacon.current_round, 1);
    assert!(beacon.rounds.is_empty());
    assert!(beacon.pending_vrf_proofs.is_empty());
    assert!(beacon.pending_partial_sigs.is_empty());
    assert!(beacon.challenges.is_empty());
    assert!(beacon.messages.is_empty());
    // Note: validators are not reset as they are configuration

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_threshold_requirement() {
    let (mut node, mut beacon) = create_beacon_node().await;

    // Register validators below threshold
    for i in 1..=2 {
        let validator = BeaconValidator {
            address: format!("validator{i}"),
            public_key: format!("pubkey{i}"),
            vrf_key: format!("vrfkey{i}"),
            stake: 1000,
            active: true,
            last_participation: None,
        };
        beacon.register_validator(validator).unwrap();
    }

    // Add VRF proofs and partial sigs below threshold
    for i in 1..=2 {
        let vrf_msg = BeaconMessageType::VrfProof {
            round: 1,
            input: "test_input".to_string(),
            proof: format!("proof_{i}"),
            output: format!("output_{i}"),
            validator: format!("validator{i}"),
            signature: format!("sig_{i}"),
            timestamp: chrono::Utc::now(),
        };
        beacon.process_vrf_proof(vrf_msg).unwrap();

        let partial_sig_msg = BeaconMessageType::PartialSignature {
            round: 1,
            validator: format!("validator{i}"),
            partial_sig: format!("partial_sig_{i}"),
            signature: format!("sig_{i}"),
            timestamp: chrono::Utc::now(),
        };
        beacon.process_partial_signature(partial_sig_msg).unwrap();
    }

    // Should not be able to finalize (below threshold of 3)
    assert!(!beacon.can_finalize_round());

    // Attempting to finalize should fail
    let result = beacon.finalize_round();
    assert!(result.is_err());

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_round_time_advance() {
    let (mut node, beacon) = create_beacon_node().await;

    // Initially should not advance (just created)
    assert!(!beacon.should_advance_round());

    // Test with very short duration
    let short_beacon = RandomnessBeaconObject::new(1, 2).unwrap(); // 1 second rounds
    sleep(Duration::from_millis(1100)).await; // Wait > 1 second

    // Now should advance
    assert!(short_beacon.should_advance_round());

    node.close().await.unwrap();
}

#[tokio::test]
async fn test_beacon_helper_functions() -> Result<()> {
    let (mut node, _beacon) = create_beacon_node().await;
    let _signer = ECDSASigner::new()?;

    // Test beacon message creation
    let message = helpers::create_vrf_proof_message(
        1,
        "test_input".to_string(),
        "test_proof".to_string(),
        "test_output".to_string(),
        "validator1".to_string(),
        &_signer,
    )?;

    // Sign and verify message
    let signature = _signer.sign(message.to_string().as_bytes())?;
    let verifier = ECDSAVerifier::new();
    let public_key_pem = _signer.get_public_key_pem()?;
    assert!(verifier.verify(message.to_string().as_bytes(), &signature, &public_key_pem)?);

    node.close().await?;
    Ok(())
}
