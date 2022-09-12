use crate::proto::raftpb::*;
use std::{collections::HashSet};
/// State of a raft peer.
#[derive(Default, Clone, Debug)]
pub struct State {
    pub term: u64,
    pub is_leader: bool,
}

impl State {
    /// The current term of this peer.
    pub fn term(&self) -> u64 {
        self.term
    }
    /// Whether this peer believes it is the leader.
    pub fn is_leader(&self) -> bool {
        self.is_leader
    }
}

#[derive(Debug)]
pub enum Event {
    ResetTimeout,
    ElectionTimeout,
    HeartBeat,
    RequestVoteReply(usize, labrpc::Result<RequestVoteReply>),
    AppendEntriesReply {
        from: usize,
        reply: labrpc::Result<AppendEntriesReply>,
        new_next_index: usize,
    },
}

#[derive(Debug)]
pub enum RoleState {
    Follower,
    Candidate {
        votes: HashSet<usize>,
    },
    Leader {
        next_index: Vec<usize>,
        match_index: Vec<usize>,
    },
}

#[derive(Debug)]
pub struct PersistentState {
    pub current_term: u64,
    pub voted_for: Option<usize>,
    pub log: Vec<Entry>,
}

impl PersistentState {
    pub fn new() -> Self {
        Self {
            current_term: 0,
            voted_for: None,
            log: vec![Default::default()], // dummy entry at index 0
        }
    }
}

// SoftState provides state that is volatile and does not need to be persisted to the WAL.
pub struct SoftState {
    pub commit_index: u64,
    pub last_applied: u64,
}

impl SoftState {
    pub fn new() -> Self {
        Self {
            commit_index: 0,
            last_applied: 0,
        }
    }
}
