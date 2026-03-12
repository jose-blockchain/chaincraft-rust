//! Slush Protocol - Avalanche paper Section 2.2
//!
//! A toy single-decree consensus protocol (NOT Byzantine fault tolerant).
//! Nodes converge on a binary choice (Red or Blue) via repeated random
//! sampling: each round, a node broadcasts its color, collects peer colors
//! from incoming messages, and flips to the majority when >= alpha*k agree.
//!
//! Implements `ApplicationObject` so it integrates with ChaincraftNode gossip.

use crate::{
    error::Result,
    shared::{MessageType, SharedMessage, SharedObjectId},
    shared_object::ApplicationObject,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::HashSet;

/// Binary color choice (paper uses R/B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Color {
    Red,
    Blue,
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Color::Red => write!(f, "R"),
            Color::Blue => write!(f, "B"),
        }
    }
}

/// A Slush vote message broadcast by a node during a round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlushVote {
    pub message_type: String,
    pub node_id: String,
    pub round: u32,
    pub color: Color,
}

/// Slush consensus state, implements ApplicationObject.
#[derive(Debug, Clone)]
pub struct SlushObject {
    id: SharedObjectId,
    pub node_id: String,
    pub color: Option<Color>,
    pub accepted: Option<Color>,
    pub k: usize,
    pub alpha: f64,
    pub m: u32,
    pub current_round: u32,
    votes: Vec<SlushVote>,
    seen_hashes: HashSet<String>,
}

impl SlushObject {
    pub fn new(node_id: String, k: usize, alpha: f64, m: u32) -> Self {
        Self {
            id: SharedObjectId::new(),
            node_id,
            color: None,
            accepted: None,
            k,
            alpha,
            m,
            current_round: 0,
            votes: Vec::new(),
            seen_hashes: HashSet::new(),
        }
    }

    /// Get all collected votes.
    pub fn votes(&self) -> &[SlushVote] {
        &self.votes
    }

    /// Count votes for a given round.
    pub fn count_votes_for_round(&self, round: u32) -> (usize, usize) {
        let mut red = 0usize;
        let mut blue = 0usize;
        for v in &self.votes {
            if v.round == round && v.node_id != self.node_id {
                match v.color {
                    Color::Red => red += 1,
                    Color::Blue => blue += 1,
                }
            }
        }
        (red, blue)
    }

    /// Process one round of Slush given collected votes. Returns whether color flipped.
    pub fn process_round(&mut self, round: u32) -> bool {
        let (red, blue) = self.count_votes_for_round(round);
        let threshold = (self.alpha * self.k as f64) as usize;
        let current = match self.color {
            Some(c) => c,
            None => return false,
        };
        if red >= threshold && current != Color::Red {
            self.color = Some(Color::Red);
            return true;
        }
        if blue >= threshold && current != Color::Blue {
            self.color = Some(Color::Blue);
            return true;
        }
        false
    }

    /// Finalize: set accepted = current color.
    pub fn finalize(&mut self) {
        self.accepted = self.color;
    }
}

/// Create a SlushVote JSON value suitable for `node.create_shared_message_with_data`.
pub fn create_vote_message(node_id: &str, round: u32, color: Color) -> Value {
    serde_json::json!({
        "message_type": "SLUSH_VOTE",
        "node_id": node_id,
        "round": round,
        "color": color,
    })
}

#[async_trait]
impl ApplicationObject for SlushObject {
    fn id(&self) -> &SharedObjectId {
        &self.id
    }

    fn type_name(&self) -> &'static str {
        "SlushObject"
    }

    async fn is_valid(&self, message: &SharedMessage) -> Result<bool> {
        let vote: std::result::Result<SlushVote, _> = serde_json::from_value(message.data.clone());
        if let Ok(v) = vote {
            return Ok(v.message_type == "SLUSH_VOTE");
        }
        Ok(false)
    }

    async fn add_message(&mut self, message: SharedMessage) -> Result<()> {
        if self.seen_hashes.contains(&message.hash) {
            return Ok(());
        }
        self.seen_hashes.insert(message.hash.clone());

        let vote: SlushVote = match serde_json::from_value(message.data.clone()) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };

        if self.color.is_none() {
            self.color = Some(vote.color);
        }

        self.votes.push(vote);
        Ok(())
    }

    fn is_merkleized(&self) -> bool {
        false
    }

    async fn get_latest_digest(&self) -> Result<String> {
        Ok(format!("{:?}", self.color))
    }

    async fn has_digest(&self, _digest: &str) -> Result<bool> {
        Ok(false)
    }

    async fn is_valid_digest(&self, _digest: &str) -> Result<bool> {
        Ok(true)
    }

    async fn add_digest(&mut self, _digest: String) -> Result<bool> {
        Ok(false)
    }

    async fn gossip_messages(&self, _digest: Option<&str>) -> Result<Vec<SharedMessage>> {
        Ok(Vec::new())
    }

    async fn get_messages_since_digest(&self, _digest: &str) -> Result<Vec<SharedMessage>> {
        Ok(Vec::new())
    }

    async fn get_state(&self) -> Result<Value> {
        Ok(serde_json::json!({
            "node_id": self.node_id,
            "color": format!("{:?}", self.color),
            "accepted": format!("{:?}", self.accepted),
            "round": self.current_round,
            "votes": self.votes.len(),
        }))
    }

    async fn reset(&mut self) -> Result<()> {
        self.color = None;
        self.accepted = None;
        self.current_round = 0;
        self.votes.clear();
        self.seen_hashes.clear();
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn ApplicationObject> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
