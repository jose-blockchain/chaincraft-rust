//! ECDSA signed transaction ledger example
//!
//! Demonstrates cryptographic primitives end-to-end: keygen, sign transfers,
//! verify, propagate over network.

use chaincraft::{
    crypto::ecdsa::ECDSASigner,
    error::Result,
    examples::ecdsa_ledger::{helpers, ECDSALedgerObject},
    network::PeerId,
    storage::MemoryStorage,
    ChaincraftNode,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    println!("Chaincraft ECDSA Ledger Example");
    println!("===============================\n");

    let id = PeerId::new();
    let storage = Arc::new(MemoryStorage::new());
    let mut node = ChaincraftNode::new(id, storage);
    node.set_port(0);
    node.add_shared_object(Box::new(ECDSALedgerObject::new()))
        .await?;
    node.start().await?;

    let alice = ECDSASigner::new()?;
    let bob = ECDSASigner::new()?;
    let alice_pk = alice.get_public_key_pem()?;
    let bob_pk = bob.get_public_key_pem()?;

    // Alice "mints" 100 (first tx from new address)
    let tx1 = helpers::create_transfer(alice_pk.clone(), bob_pk.clone(), 50, 0, &alice)?;
    node.create_shared_message_with_data(tx1).await?;
    println!("Alice -> Bob: 50 (first tx = mint)");
    sleep(Duration::from_millis(200)).await;

    // Bob -> Alice: 20
    let tx2 = helpers::create_transfer(bob_pk.clone(), alice_pk.clone(), 20, 0, &bob)?;
    node.create_shared_message_with_data(tx2).await?;
    println!("Bob -> Alice: 20");
    sleep(Duration::from_millis(200)).await;

    let objs = node.shared_objects().await;
    if let Some(obj) = objs.first() {
        if let Some(ledger) = obj.as_any().downcast_ref::<ECDSALedgerObject>() {
            println!("\nLedger entries: {}", ledger.entries().len());
            println!("Alice balance: {}", ledger.balance(&alice_pk));
            println!("Bob balance: {}", ledger.balance(&bob_pk));
        }
    }

    node.close().await?;
    println!("Done.");
    Ok(())
}
