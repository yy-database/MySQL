#![warn(missing_docs)]

use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use yyds_types::{DsError, DsResult, YYError};
use yykv_event::WalEvent;
use uuid::Uuid;

use std::collections::HashMap;
use tokio::sync::mpsc;

/// Consensus roles in the cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeRole {
    Follower,
    Candidate,
    Leader,
}

/// Consensus messages for replication and voting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusMessage {
    RequestVote {
        term: u64,
        candidate_id: Uuid,
        last_log_index: u64,
        last_log_term: u64,
    },
    VoteResponse {
        term: u64,
        vote_granted: bool,
    },
    AppendEntries {
        term: u64,
        leader_id: Uuid,
        prev_log_index: u64,
        prev_log_term: u64,
        entries: Vec<WalEvent>,
        leader_commit: u64,
    },
    AppendResponse {
        term: u64,
        success: bool,
        last_index: u64,
        node_id: Uuid,
    },
}

/// ConsensusAdapter allows switching between different consensus protocols (e.g., Raft, Gossip).
#[async_trait]
pub trait ConsensusAdapter: Send + Sync {
    async fn propose(&self, event: WalEvent) -> DsResult<()>;
    async fn step(&self, msg: ConsensusMessage) -> DsResult<()>;
    async fn role(&self) -> NodeRole;
    async fn term(&self) -> u64;
}

/// Gossip implementation of the ConsensusAdapter for eventual consistency.
pub struct GossipAdapter {
    _id: Uuid,
    peers: tokio::sync::RwLock<Vec<Uuid>>,
    // Gossip-specific state like version vectors or timestamps
}

impl GossipAdapter {
    pub fn new(id: Uuid, peers: Vec<Uuid>) -> Self {
        Self {
            _id: id,
            peers: tokio::sync::RwLock::new(peers),
        }
    }

    pub async fn add_peer(&self, peer: Uuid) {
        let mut peers = self.peers.write().await;
        if !peers.contains(&peer) {
            peers.push(peer);
        }
    }
}

#[async_trait]
impl ConsensusAdapter for GossipAdapter {
    async fn propose(&self, _event: WalEvent) -> DsResult<()> {
        // Implement epidemic propagation
        Ok(())
    }

    async fn step(&self, _msg: ConsensusMessage) -> DsResult<()> {
        // Handle gossip messages
        Ok(())
    }

    async fn role(&self) -> NodeRole {
        NodeRole::Follower // Gossip nodes are typically peers
    }

    async fn term(&self) -> u64 {
        0 // Gossip doesn't use terms like Raft
    }
}

/// Raft implementation of the ConsensusAdapter.
pub struct RaftAdapter {
    id: Uuid,
    peers: Vec<Uuid>,
    state: tokio::sync::RwLock<RaftState>,
    msg_tx: mpsc::Sender<(Uuid, ConsensusMessage)>,
}

/// Internal state of the Raft node.
pub struct RaftState {
    /// Current role of the node.
    pub role: NodeRole,
    /// Current term of the node.
    pub current_term: u64,
    /// Who the node voted for in the current term.
    pub voted_for: Option<Uuid>,
    /// Log entries.
    pub log: Vec<WalEvent>,
    /// Index of the highest log entry known to be committed.
    pub commit_index: u64,
    /// Index of the highest log entry applied to the state machine.
    pub _last_applied: u64,
    /// For each server, index of the next log entry to send to that server.
    pub next_index: HashMap<Uuid, u64>,
    /// For each server, index of the highest log entry known to be replicated on that server.
    pub match_index: HashMap<Uuid, u64>,
    /// Votes received in the current term.
    pub votes_received: HashMap<Uuid, bool>,
}

impl RaftAdapter {
    /// Create a new RaftAdapter.
    pub fn new(id: Uuid, peers: Vec<Uuid>, msg_tx: mpsc::Sender<(Uuid, ConsensusMessage)>) -> Self {
        Self {
            id,
            peers,
            state: tokio::sync::RwLock::new(RaftState {
                role: NodeRole::Follower,
                current_term: 0,
                voted_for: None,
                log: Vec::new(),
                commit_index: 0,
                _last_applied: 0,
                next_index: HashMap::new(),
                match_index: HashMap::new(),
                votes_received: HashMap::new(),
            }),
            msg_tx,
        }
    }

    /// Access the internal state of the Raft node.
    pub fn state(&self) -> &tokio::sync::RwLock<RaftState> {
        &self.state
    }

    /// Internal function to become a candidate and start election
    pub async fn start_election(&self) -> DsResult<()> {
        let mut state = self.state.write().await;
        state.role = NodeRole::Candidate;
        state.current_term += 1;
        state.voted_for = Some(self.id);
        state.votes_received.clear();
        state.votes_received.insert(self.id, true);

        let last_log_index = state.log.len() as u64;
        let last_log_term = if last_log_index > 0 {
            // Simplified: in real raft we need to track terms per log entry
            state.current_term
        } else {
            0
        };

        for peer in &self.peers {
            let msg = ConsensusMessage::RequestVote {
                term: state.current_term,
                candidate_id: self.id,
                last_log_index,
                last_log_term,
            };
            let _ = self.msg_tx.send((*peer, msg)).await;
        }

        Ok(())
    }
}

#[async_trait]
impl ConsensusAdapter for RaftAdapter {
    async fn propose(&self, event: WalEvent) -> DsResult<()> {
        let mut state = self.state.write().await;
        if state.role != NodeRole::Leader {
            return Err(DsError::execution("Not a leader"));
        }

        // Add to local log
        state.log.push(event);
        let current_index = state.log.len() as u64;
        state.match_index.insert(self.id, current_index);

        // Trigger replication to peers
        for peer in &self.peers {
            let prev_index = state.next_index.get(peer).cloned().unwrap_or(1) - 1;
            let entries = if current_index >= state.next_index.get(peer).cloned().unwrap_or(1) {
                state.log[(prev_index as usize)..].to_vec()
            } else {
                vec![]
            };

            let msg = ConsensusMessage::AppendEntries {
                term: state.current_term,
                leader_id: self.id,
                prev_log_index: prev_index,
                prev_log_term: state.current_term, // Simplified
                entries,
                leader_commit: state.commit_index,
            };
            let _ = self.msg_tx.send((*peer, msg)).await;
        }

        Ok(())
    }

    async fn step(&self, msg: ConsensusMessage) -> DsResult<()> {
        let mut state = self.state.write().await;
        
        match msg {
            ConsensusMessage::RequestVote { term, candidate_id, .. } => {
                if term > state.current_term {
                    state.current_term = term;
                    state.role = NodeRole::Follower;
                    state.voted_for = None;
                }

                let can_vote = state.voted_for.is_none() || state.voted_for == Some(candidate_id);
                if term == state.current_term && can_vote {
                    state.voted_for = Some(candidate_id);
                    let _ = self.msg_tx.send((candidate_id, ConsensusMessage::VoteResponse {
                        term: state.current_term,
                        vote_granted: true,
                    })).await;
                } else {
                    let _ = self.msg_tx.send((candidate_id, ConsensusMessage::VoteResponse {
                        term: state.current_term,
                        vote_granted: false,
                    })).await;
                }
            }
            ConsensusMessage::VoteResponse { term, vote_granted } => {
                if term > state.current_term {
                    state.current_term = term;
                    state.role = NodeRole::Follower;
                    return Ok(());
                }

                if state.role == NodeRole::Candidate && term == state.current_term && vote_granted {
                    // Simplified: counting votes
                    // In real implementation we need to track WHICH node voted
                    // For now we just use a placeholder to represent "another vote"
                    let vote_count = state.votes_received.len() + 1;
                    if vote_count > (self.peers.len() + 1) / 2 {
                        state.role = NodeRole::Leader;
                        // Initialize leader state
                        let last_index = state.log.len() as u64;
                        for peer in &self.peers {
                            state.next_index.insert(*peer, last_index + 1);
                            state.match_index.insert(*peer, 0);
                        }
                    }
                }
            }
            ConsensusMessage::AppendEntries { term, entries, .. } => {
                if term >= state.current_term {
                    state.current_term = term;
                    state.role = NodeRole::Follower;
                    // Append entries to log
                    for entry in entries {
                        state.log.push(entry);
                    }
                }
            }
            ConsensusMessage::AppendResponse { term, success, last_index, node_id } => {
                if term > state.current_term {
                    state.current_term = term;
                    state.role = NodeRole::Follower;
                    return Ok(());
                }

                if state.role == NodeRole::Leader && term == state.current_term {
                    if success {
                        state.match_index.insert(node_id, last_index);
                        state.next_index.insert(node_id, last_index + 1);
                        
                        // Check if we can commit
                        // Simplified: check if majority match_index >= N
                    } else {
                        let current_next = state.next_index.get(&node_id).cloned().unwrap_or(1);
                        if current_next > 1 {
                            state.next_index.insert(node_id, current_next - 1);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn role(&self) -> NodeRole {
        self.state.read().await.role
    }

    async fn term(&self) -> u64 {
        self.state.read().await.current_term
    }
}