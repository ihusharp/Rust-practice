use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::channel::oneshot;
use futures::executor::ThreadPool;
use futures::task::SpawnExt;
use futures::{select, FutureExt, StreamExt};
use rand::Rng;

#[cfg(test)]
pub mod config;
pub mod errors;
pub mod persister;
mod states;
#[cfg(test)]
mod tests;

use self::errors::*;
use self::persister::*;
use self::states::*;
use crate::proto::raftpb::*;

pub use self::states::State;

/// As each Raft peer becomes aware that successive log entries are committed,
/// the peer should send an `ApplyMsg` to the service (or tester) on the same
/// server, via the `apply_ch` passed to `Raft::new`.
#[derive(Debug)]
pub enum ApplyMsg {
    Command {
        data: Vec<u8>,
        index: u64,
    },
    // For 2D:
    Snapshot {
        data: Vec<u8>,
        term: u64,
        index: u64,
    },
}

// A single Raft peer.
pub struct Raft {
    // RPC end points of all peers
    peers: Vec<RaftClient>,
    // Object to hold this peer's persisted state
    persister: Box<dyn Persister>,
    // this peer's index into peers[]
    me: usize,
    // Your data here (2A, 2B, 2C).
    // Look at the paper's Figure 2 for a description of what
    // state a Raft server must maintain.
    role: RoleState,
    hard_state: PersistentState,
    soft_state: SoftState,

    // channels
    event_loop_tx: Option<UnboundedSender<Event>>, // should always be Some
    apply_ch: UnboundedSender<ApplyMsg>,

    executor: ThreadPool,
}

impl Raft {
    /// the service or tester wants to create a Raft server. the ports
    /// of all the Raft servers (including this one) are in peers. this
    /// server's port is peers[me]. all the servers' peers arrays
    /// have the same order. persister is a place for this server to
    /// save its persistent state, and also initially holds the most
    /// recent saved state, if any. apply_ch is a channel on which the
    /// tester or service expects Raft to send ApplyMsg messages.
    /// This method must return quickly.
    pub fn new(
        peers: Vec<RaftClient>,
        me: usize,
        persister: Box<dyn Persister>,
        apply_ch: UnboundedSender<ApplyMsg>,
    ) -> Raft {
        let raft_state = persister.raft_state();

        // Your initialization code here (2A, 2B, 2C).
        let mut rf = Raft {
            peers,
            persister,
            me,
            role: RoleState::Follower,
            hard_state: PersistentState::new(),
            soft_state: SoftState::new(),
            event_loop_tx: None,
            executor: ThreadPool::new().unwrap(),
            apply_ch,
        };

        // initialize from state persisted before a crash
        rf.restore(&raft_state);

        rf.turn_follower();

        rf
    }

    fn start<M>(&mut self, command: &M) -> Result<(u64, u64)>
    where
        M: labcodec::Message,
    {
        match &self.role {
            RoleState::Leader { .. } => {
                let mut data = Vec::new();
                labcodec::encode(command, &mut data).map_err(Error::Encode)?;
                let entry = Entry {
                    term: self.hard_state.current_term,
                    data,
                };
                self.hard_state.log.push(entry);
                Ok((self.last_log_index(), self.last_log_term()))
            }
            _ => Err(Error::NotLeader),
        }
    }

    fn event_loop_tx(&self) -> &UnboundedSender<Event> {
        self.event_loop_tx.as_ref().expect("no event loop sender")
    }

    fn cond_install_snapshot(
        &mut self,
        last_included_term: u64,
        last_included_index: u64,
        snapshot: &[u8],
    ) -> bool {
        // Your code here (2D).
        crate::your_code_here((last_included_term, last_included_index, snapshot));
    }

    fn snapshot(&mut self, index: u64, snapshot: &[u8]) {
        // Your code here (2D).
        crate::your_code_here((index, snapshot));
    }

    fn start_election(&mut self) {
        println!("[start_election! {}]", self.me);
        for (i, peer) in self.other_peers() {
            let tx = self.event_loop_tx().clone();
            let fut = peer.request_vote(&self.request_vote_args());
            self.executor
                .spawn(async move {
                    let reply = fut.await;
                    let _ = tx
                        .unbounded_send(Event::RequestVoteReply(i, reply))
                        .unwrap();
                })
                .unwrap();
        }
    }
}

// state actions
impl Raft {
    fn update_term(&mut self, term: u64) {
        self.hard_state.current_term = term;
        self.hard_state.voted_for = None;
    }

    fn turn_leader(&mut self) {
        let next_index = vec![self.hard_state.log.len(); self.peers.len()];
        let match_index = vec![0; self.peers.len()];
        self.role = RoleState::Leader {
            next_index,
            match_index,
        }
    }

    fn turn_candidate(&mut self) {
        let votes = [self.me].iter().cloned().collect();
        self.role = RoleState::Candidate { votes };
    }

    fn turn_follower(&mut self) {
        self.role = RoleState::Follower;
    }

    /// save Raft's persistent state to stable storage,
    /// where it can later be retrieved after a crash and restart.
    /// see paper's Figure 2 for a description of what should be persistent.
    fn persist(&mut self) {
        // Your code here (2C).
        let mut data = Vec::new();
        labcodec::encode(&self.hard_state, &mut data).unwrap();
        self.persister.save_raft_state(data);
    }

    /// restore previously persisted state.
    fn restore(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        // Your code here (2C).
        self.hard_state = labcodec::decode(data).unwrap();
    }

    fn last_log_term(&self) -> u64 {
        self.hard_state.log.last().unwrap().term
    }

    fn last_log_index(&self) -> u64 {
        (self.hard_state.log.len() - 1) as u64
    }

    /// Determine whether the prev log of the AppendEntries RPC Caller
    /// matches the current node's log.
    /// If match returns true, otherwise it returns false.
    ///
    /// `args_prev_term` is the log term that Caller wants to match
    /// `args_prev_index` is the log index that Caller wants to match
    fn is_match(&self, args_prev_term: u64, args_prev_index: u64) -> bool {
        return self
            .hard_state
            .log
            .get(args_prev_index as usize)
            .map(|e| e.term)
            == Some(args_prev_term);
    }
}

// assit functions
impl Raft {
    fn other_peers(&self) -> impl Iterator<Item = (usize, &RaftClient)> {
        self.peers
            .iter()
            .enumerate()
            .filter(move |(i, _)| i != &self.me)
    }

    fn commit_to_new_index(&mut self, new_index: u64) {
        if new_index <= self.soft_state.commit_index {
            return;
        }

        for i in self.soft_state.commit_index + 1..=new_index {
            let msg = ApplyMsg::Command {
                data: self.hard_state.log[i as usize].data.clone(),
                index: i,
            };
            self.apply_ch.unbounded_send(msg).unwrap();
        }
        self.soft_state.commit_index = new_index as u64;
    }

    // maybe_commit attempts to advance the commit index. Returns true if
    // the commit index changed.
    fn maybe_commit(&mut self) {
        if let RoleState::Leader { match_index, .. } = &self.role {
            let mut new_commit_index = self.soft_state.commit_index;

            // Find the max match index.
            for cur_index in self.soft_state.commit_index..self.hard_state.log.len() as u64 {
                let quorum_cnt = self
                    .other_peers()
                    .filter(|(i, _)| match_index[*i] >= cur_index as usize)
                    .count()
                    + 1;

                let is_current_term =
                    self.hard_state.log[cur_index as usize].term == self.hard_state.current_term;
                if is_current_term {
                    if quorum_cnt > self.peers.len() / 2 {
                        new_commit_index = cur_index;
                    } else {
                        break;
                    }
                }
            }

            println!("[maybe_commit], new commit index is: {}", new_commit_index);
            self.commit_to_new_index(new_commit_index);
        }
    }
}

impl Raft {
    fn request_vote_args(&self) -> RequestVoteArgs {
        RequestVoteArgs {
            term: self.hard_state.current_term,
            candidate_id: self.me as u64,
            last_log_index: self.last_log_index(),
            last_log_term: self.last_log_term(),
        }
    }

    fn append_entries_args(&self, start_at: usize) -> AppendEntriesArgs {
        let entries = self.hard_state.log[start_at..].iter().cloned().collect();
        let prev_log_index = start_at - 1;

        AppendEntriesArgs {
            term: self.hard_state.current_term,
            leader_id: self.me as u64,
            prev_log_index: prev_log_index as u64,
            prev_log_term: self.hard_state.log[prev_log_index].term,
            entries,
            leader_commit_index: self.soft_state.commit_index as u64,
        }
    }
}

// event actions
impl Raft {
    fn schedule_event(&mut self, event: Event) {
        if let Err(e) = self.event_loop_tx().unbounded_send(event) {
            error!("schedule event: {}", e);
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::ResetTimeout => unreachable!(), // already handled by timer
            Event::ElectionTimeout => self.handle_election_timeout(),
            Event::HeartBeat => self.handle_heartbeat(),
            Event::RequestVoteReply(from, reply) => self.handle_request_vote_reply(from, reply),
            Event::AppendEntriesReply {
                from,
                reply,
                new_next_index,
            } => self.handle_append_entries_reply(from, reply, new_next_index),
            Event::ForcePersist => self.persist(),
        }
    }

    fn handle_election_timeout(&mut self) {
        match self.role {
            RoleState::Follower | RoleState::Candidate { .. } => {
                // start new election
                self.turn_candidate();
                self.update_term(self.hard_state.current_term + 1);
                self.hard_state.voted_for = Some(self.me as u64);

                self.schedule_event(Event::ResetTimeout);
                self.start_election();
            }
            _ => {} // no timeout for leader
        }
    }

    // for leader to make sure peers are still alive
    fn handle_heartbeat(&mut self) {
        match self.role {
            RoleState::Leader { .. } => self.heart_beat_sync_log(),
            _ => {} // no heartbeat for follower and candidate
        }
        self.persist();
    }

    // sync log from leader to follower when heartbeat
    fn heart_beat_sync_log(&mut self) {
        if let RoleState::Leader { next_index, .. } = &self.role {
            for (i, peer) in self.other_peers() {
                let tx = self.event_loop_tx().clone();
                let args = self.append_entries_args(next_index[i]);
                let fut = peer.append_entries(&args);
                let new_next_index = self.hard_state.log.len();

                self.executor
                    .spawn(async move {
                        let reply = fut.await;
                        let _ = tx.unbounded_send(Event::AppendEntriesReply {
                            from: i,
                            reply,
                            new_next_index,
                        });
                    })
                    .unwrap();
            }
        }
    }

    fn handle_request_vote_request(
        &mut self,
        args: RequestVoteArgs,
    ) -> labrpc::Result<RequestVoteReply> {
        println!("[handle_request_vote_request! {}] {:?}", self.me, args);
        let vote_granted = {
            if self.hard_state.current_term > args.term {
                None
            } else {
                if args.term > self.hard_state.current_term {
                    self.update_term(args.term);
                    self.turn_follower()
                }
                let voted_id = args.candidate_id;
                // if self is candidate, then voted_for is already Some(me)
                let not_voted_other =
                    self.hard_state.voted_for.map(|v| v == voted_id) != Some(false);
                // cand's log must be more up-to-date
                let cand_up_to_date = (args.last_log_term, args.last_log_index)
                    >= (self.last_log_term(), self.last_log_index());

                if not_voted_other && cand_up_to_date {
                    Some(voted_id)
                } else {
                    None
                }
            }
        };

        Ok(RequestVoteReply {
            term: self.hard_state.current_term,
            vote_granted: vote_granted.is_some(),
        })
    }

    fn handle_request_vote_reply(&mut self, from: usize, reply: labrpc::Result<RequestVoteReply>) {
        println!("[handle_request_vote_reply! {}] {:?}", self.me, reply);
        match reply {
            Ok(reply) => {
                if reply.term > self.hard_state.current_term {
                    self.update_term(reply.term);
                    self.turn_follower();
                }

                if let RoleState::Candidate { votes } = &mut self.role {
                    if reply.vote_granted && reply.term == self.hard_state.current_term {
                        votes.insert(from);
                        if votes.len() > self.peers.len() / 2 {
                            self.turn_leader();
                            self.schedule_event(Event::HeartBeat);
                        }
                    }
                }
            }
            Err(err) => {
                println!("request vote -> err: {}", err)
            }
        }
    }

    fn handle_append_entries_request(
        &mut self,
        args: AppendEntriesArgs,
    ) -> labrpc::Result<AppendEntriesReply> {
        println!("[handle_append_entries! id: {}] {:?}", self.me, args);
        // let mut index = self.hard_state.log.len() as u64;
        let success = {
            if self.hard_state.current_term > args.term {
                // index = self.soft_state.commit_index;
                false
            } else {
                if args.term > self.hard_state.current_term
                    || matches!(self.role, RoleState::Candidate { .. })
                        && args.term == self.hard_state.current_term
                {
                    self.update_term(args.term);
                    self.turn_follower();
                }

                match self.role {
                    RoleState::Follower => {
                        self.schedule_event(Event::ResetTimeout);

                        // log replication
                        // if there is no conflict, append entries
                        // firstly make sure prevLogTerm and prevLogIndex match
                        if !self.is_match(args.prev_log_term, args.prev_log_index) {
                            false
                        } else {
                            // delete all entries after prevLogIndex
                            while self.hard_state.log.len() as u64 > args.prev_log_index + 1 {
                                self.hard_state.log.pop();
                            }
                            // append entries
                            self.hard_state.log.extend(args.entries);

                            println!(
                                "[handle_append_entries need to commit! id: {}]  now log is: {:?}",
                                self.me, self.hard_state.log
                            );
                            if args.leader_commit_index > self.soft_state.commit_index {
                                let new_commit_index = args
                                    .leader_commit_index
                                    .min(self.hard_state.log.len() as u64 - 1);
                                self.commit_to_new_index(new_commit_index);
                            }
                            true
                        }
                    }
                    RoleState::Candidate { .. } => {
                        unreachable!("candidate should turn into follower before")
                    }
                    RoleState::Leader { .. } => unreachable!("another leader with same term found"),
                }
            }
        };

        Ok(AppendEntriesReply {
            term: self.hard_state.current_term,
            success,
        })
    }

    fn handle_append_entries_reply(
        &mut self,
        from: usize,
        reply: labrpc::Result<AppendEntriesReply>,
        index: usize,
    ) {
        println!(
            "[handle_append_entries_reply! id: {}] index: {}, reply: {:?}",
            self.me, index, reply
        );
        match reply {
            Ok(reply) => {
                if reply.term > self.hard_state.current_term {
                    self.update_term(reply.term);
                    self.turn_follower();
                }

                if let RoleState::Leader {
                    next_index,
                    match_index,
                } = &mut self.role
                {
                    if reply.success {
                        match_index[from] = index - 1;
                        next_index[from] = index;
                        self.maybe_commit();
                    } else {
                        next_index[from] = next_index[from].saturating_sub(1);
                    }
                };
            }
            Err(err) => {
                println!("[handle_append_entries_reply] err is: {}", err);
            }
        }
    }
}

#[derive(Clone)]
pub struct Node {
    // Your code here.
    raft: Arc<Mutex<Raft>>,
    event_loop_tx: UnboundedSender<Event>,
    shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    executor: ThreadPool,
}

impl Node {
    /// Create a new raft service.
    pub fn new(mut raft: Raft) -> Node {
        // Your code here.
        let (event_loop_tx, event_loop_rx) = mpsc::unbounded();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        raft.event_loop_tx = Some(event_loop_tx.clone());

        let node = Node {
            raft: Arc::new(Mutex::new(raft)),
            event_loop_tx,
            shutdown_tx: Arc::new(Mutex::new(Some(shutdown_tx))),
            executor: ThreadPool::new().unwrap(),
        };
        node.start_event_loop(event_loop_rx, shutdown_rx);

        node
    }

    fn start_event_loop(
        &self,
        mut event_loop_rx: UnboundedReceiver<Event>,
        mut shutdown_rx: oneshot::Receiver<()>,
    ) {
        let raft = Arc::clone(&self.raft);
        let event_loop_tx = self.event_loop_tx.clone();

        self.executor
            .spawn(async move {
                // We suggest ElectionTick = 10 * HeartbeatTick to avoid
                // unnecessary leader switching.
                let build_rand_timeout_timer = || {
                    futures_timer::Delay::new(Duration::from_millis(
                        rand::thread_rng().gen_range(1000, 1500),
                    ))
                    .fuse()
                };
                let build_heartbeat_timer =
                    || futures_timer::Delay::new(Duration::from_millis(100)).fuse();

                let mut timeout_timer = build_rand_timeout_timer();
                let mut heartbeat_timer = build_heartbeat_timer();

                loop {
                    select! {
                        event = event_loop_rx.select_next_some() => {
                            match event {
                                Event::ResetTimeout =>  timeout_timer = build_rand_timeout_timer(),
                                event => raft.lock().unwrap().handle_event(event),
                            }
                        }
                        _ = timeout_timer => {
                            event_loop_tx.unbounded_send(Event::ElectionTimeout).unwrap();
                            timeout_timer = build_rand_timeout_timer();
                        }
                        _ = heartbeat_timer => {
                            event_loop_tx.unbounded_send(Event::HeartBeat).unwrap();
                            heartbeat_timer = build_heartbeat_timer();
                        }
                        _ = shutdown_rx => {
                            let mut raft = raft.lock().unwrap();
                            raft.handle_event(Event::ForcePersist);
                        }
                    }
                }
            })
            .expect("failed to spawn event loop");
    }

    /// the service using Raft (e.g. a k/v server) wants to start
    /// agreement on the next command to be appended to Raft's log. if this
    /// server isn't the leader, returns [`Error::NotLeader`]. otherwise start
    /// the agreement and return immediately. there is no guarantee that this
    /// command will ever be committed to the Raft log, since the leader
    /// may fail or lose an election. even if the Raft instance has been killed,
    /// this function should return gracefully.
    ///
    /// the first value of the tuple is the index that the command will appear
    /// at if it's ever committed. the second is the current term.
    ///
    /// This method must return without blocking on the raft.
    pub fn start<M>(&self, command: &M) -> Result<(u64, u64)>
    where
        M: labcodec::Message,
    {
        // Your code here.
        // Example:
        // self.raft.start(command)
        self.raft.lock().unwrap().start(command)
    }

    /// The current term of this peer.
    pub fn term(&self) -> u64 {
        // Your code here.
        // Example:
        // self.raft.term
        self.raft.lock().unwrap().hard_state.current_term
    }

    /// Whether this peer believes it is the leader.
    pub fn is_leader(&self) -> bool {
        // Your code here.
        // Example:
        // self.raft.leader_id == self.id
        matches!(self.raft.lock().unwrap().role, RoleState::Leader { .. })
    }

    /// The current state of this peer.
    pub fn get_state(&self) -> State {
        State {
            term: self.term(),
            is_leader: self.is_leader(),
        }
    }

    /// the tester calls kill() when a Raft instance won't be
    /// needed again. you are not required to do anything in
    /// kill(), but it might be convenient to (for example)
    /// turn off debug output from this instance.
    /// In Raft paper, a server crash is a PHYSICAL crash,
    /// A.K.A all resources are reset. But we are simulating
    /// a VIRTUAL crash in tester, so take care of background
    /// threads you generated with this Raft Node.
    pub fn kill(&self) {
        // Your code here, if desired.
        if let Some(tx) = self.shutdown_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }

    /// A service wants to switch to snapshot.  
    ///
    /// Only do so if Raft hasn't have more recent info since it communicate
    /// the snapshot on `apply_ch`.
    pub fn cond_install_snapshot(
        &self,
        last_included_term: u64,
        last_included_index: u64,
        snapshot: &[u8],
    ) -> bool {
        // Your code here.
        // Example:
        // self.raft.cond_install_snapshot(last_included_term, last_included_index, snapshot)
        crate::your_code_here((last_included_term, last_included_index, snapshot));
    }

    /// The service says it has created a snapshot that has all info up to and
    /// including index. This means the service no longer needs the log through
    /// (and including) that index. Raft should now trim its log as much as
    /// possible.
    pub fn snapshot(&self, index: u64, snapshot: &[u8]) {
        // Your code here.
        // Example:
        // self.raft.snapshot(index, snapshot)
        crate::your_code_here((index, snapshot));
    }
}

#[async_trait::async_trait]
impl RaftService for Node {
    // example RequestVote RPC handler.
    //
    // CAVEATS: Please avoid locking or sleeping here, it may jam the network.
    async fn request_vote(&self, args: RequestVoteArgs) -> labrpc::Result<RequestVoteReply> {
        let mut raft = self.raft.lock().unwrap();
        raft.handle_request_vote_request(args)
    }

    // example AppendEntries RPC handler.
    //
    // CAVEATS: Please avoid locking or sleeping here, it may jam the network.
    async fn append_entries(&self, args: AppendEntriesArgs) -> labrpc::Result<AppendEntriesReply> {
        let mut raft = self.raft.lock().unwrap();
        raft.handle_append_entries_request(args)
    }
}
