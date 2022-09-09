use std::collections::HashSet;
use std::sync::mpsc::{sync_channel, Receiver};
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
#[cfg(test)]
mod tests;

use self::errors::*;
use self::persister::*;
use crate::proto::raftpb::*;

/// As each Raft peer becomes aware that successive log entries are committed,
/// the peer should send an `ApplyMsg` to the service (or tester) on the same
/// server, via the `apply_ch` passed to `Raft::new`.
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
    Timeout,
    HeartBeat,
    RequestVoteReply(usize, RequestVoteReply),
    AppendEntriesReply(usize, AppendEntriesReply),
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
    current_term: u64,
    voted_for: Option<usize>,
    log: Vec<(u64, Entry)>,
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

#[derive(Debug)]
// SoftState provides state that is volatile and does not need to be persisted to the WAL.
pub struct SoftState {
    commit_index: u64,
    last_applied: u64,
}

impl SoftState {
    pub fn new() -> Self {
        Self {
            commit_index: 0,
            last_applied: 0,
        }
    }
}

// A single Raft peer.
pub struct Raft {
    // RPC end points of all peers
    peers: Vec<RaftClient>,
    // Object to hold this peer's persisted state
    persister: Box<dyn Persister>,
    // this peer's index into peers[]
    me: usize,
    state: Arc<State>,
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
            state: Arc::default(),
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

    fn event_loop_tx(&self) -> &UnboundedSender<Event> {
        self.event_loop_tx.as_ref().expect("no event loop sender")
    }

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
        // Example:
        // labcodec::encode(&self.xxx, &mut data).unwrap();
        // labcodec::encode(&self.yyy, &mut data).unwrap();
        // self.persister.save_raft_state(data);
    }

    /// restore previously persisted state.
    fn restore(&mut self, data: &[u8]) {
        if data.is_empty() {
            // bootstrap without any state?
        }
        // Your code here (2C).
        // Example:
        // match labcodec::decode(data) {
        //     Ok(o) => {
        //         self.xxx = o.xxx;
        //         self.yyy = o.yyy;
        //     }
        //     Err(e) => {
        //         panic!("{:?}", e);
        //     }
        // }
    }

    /// example code to send a RequestVote RPC to a server.
    /// server is the index of the target server in peers.
    /// expects RPC arguments in args.
    ///
    /// The labrpc package simulates a lossy network, in which servers
    /// may be unreachable, and in which requests and replies may be lost.
    /// This method sends a request and waits for a reply. If a reply arrives
    /// within a timeout interval, This method returns Ok(_); otherwise
    /// this method returns Err(_). Thus this method may not return for a while.
    /// An Err(_) return can be caused by a dead server, a live server that
    /// can't be reached, a lost request, or a lost reply.
    ///
    /// This method is guaranteed to return (perhaps after a delay) *except* if
    /// the handler function on the server side does not return.  Thus there
    /// is no need to implement your own timeouts around this method.
    ///
    /// look at the comments in ../labrpc/src/lib.rs for more details.
    fn send_request_vote(
        &self,
        server: usize,
        args: RequestVoteArgs,
    ) -> Receiver<Result<RequestVoteReply>> {
        // Your code here if you want the rpc becomes async.
        // Example:
        // ```
        // let peer = &self.peers[server];
        // let peer_clone = peer.clone();
        // let (tx, rx) = channel();
        // peer.spawn(async move {
        //     let res = peer_clone.request_vote(&args).await.map_err(Error::Rpc);
        //     tx.send(res);
        // });
        // rx
        // ```
        let (tx, rx) = sync_channel::<Result<RequestVoteReply>>(1);
        crate::your_code_here((server, args, tx, rx))
    }

    fn start<M>(&self, command: &M) -> Result<(u64, u64)>
    where
        M: labcodec::Message,
    {
        let index = 0;
        let term = 0;
        let is_leader = true;
        let mut buf = vec![];
        labcodec::encode(command, &mut buf).map_err(Error::Encode)?;
        // Your code here (2B).

        if is_leader {
            Ok((index, term))
        } else {
            Err(Error::NotLeader)
        }
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
        for (i, peer) in self.peers.iter().enumerate() {
            if i == self.me {
                continue;
            }
            let tx = self.event_loop_tx().clone();
            let fut = peer.request_vote(&self.request_vote_args());
            self.executor
                .spawn(async move {
                    if let Ok(reply) = fut.await {
                        tx.unbounded_send(Event::RequestVoteReply(i, reply))
                            .unwrap();
                        // todo: rx may be closed
                    }
                })
                .unwrap();
        }
    }
}

impl Raft {
    fn request_vote_args(&self) -> RequestVoteArgs {
        RequestVoteArgs {
            term: self.hard_state.current_term,
            candidate_id: self.me as u64,
            last_log_index: self.hard_state.log.len() as u64 - 1,
            last_log_term: self.hard_state.log.last().unwrap().0,
        }
    }

    fn append_entries_args(&self) -> AppendEntriesArgs {
        AppendEntriesArgs {
            term: self.hard_state.current_term,
            leader_id: self.me as u64,
            prev_log_index: self.hard_state.log.len() as u64 - 1,
            prev_log_term: self.hard_state.log.last().unwrap().0,
            entries: vec![],
            leader_commit_index: self.soft_state.commit_index,
        }
    }
}

impl Raft {
    fn schedule_event(&mut self, event: Event) {
        if let Err(e) = self.event_loop_tx().unbounded_send(event) {
            error!("schedule event: {}", e);
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::ResetTimeout => unreachable!(), // already handled by timer
            Event::Timeout => self.handle_timeout(),
            Event::HeartBeat => self.handle_heartbeat(),
            Event::RequestVoteReply(from, reply) => self.handle_request_vote_reply(from, reply),
            Event::AppendEntriesReply(from, reply) => self.handle_append_entries_reply(from, reply),
        }
    }

    fn handle_timeout(&mut self) {
        match self.role {
            RoleState::Follower | RoleState::Candidate { .. } => {
                // start new election
                self.turn_candidate();
                self.update_term(self.hard_state.current_term + 1);
                self.hard_state.voted_for = Some(self.me);

                self.schedule_event(Event::ResetTimeout);
                self.start_election();
            }
            _ => {} // no timeout for leader
        }
    }

    // for leader to make sure peers are still alive
    fn handle_heartbeat(&mut self) {
        match self.role {
            RoleState::Leader { .. } => self.append_entries(),
            _ => {} // no heartbeat for follower and candidate
        }
    }

    fn handle_request_vote_request(
        &mut self,
        args: RequestVoteArgs,
    ) -> labrpc::Result<RequestVoteReply> {
        println!("[handle_request_vote_request! {}] {:?}", self.me, args);
        let vote_granted = {
            if self.hard_state.current_term > args.term {
                false
            } else {
                if args.term > self.hard_state.current_term {
                    self.update_term(args.term);
                    self.role = RoleState::Follower;
                }
                let voted_id = args.candidate_id as usize;
                if self.hard_state.voted_for.is_none()
                    || self.hard_state.voted_for == Some(voted_id)
                {
                    self.hard_state.voted_for = Some(voted_id);
                    true
                } else {
                    false
                }
            }
        };

        Ok(RequestVoteReply {
            term: self.hard_state.current_term,
            vote_granted,
        })
    }

    fn handle_request_vote_reply(&mut self, from: usize, reply: RequestVoteReply) {
        println!("[handle_request_vote_reply! {}] {:?}", self.me, reply);
        if reply.term > self.hard_state.current_term {
            self.update_term(reply.term);
            self.role = RoleState::Follower;
        }

        match &mut self.role {
            RoleState::Candidate { votes } => {
                if reply.vote_granted && reply.term == self.hard_state.current_term {
                    votes.insert(from);
                    if votes.len() > self.peers.len() / 2 {
                        self.turn_leader();
                        self.schedule_event(Event::HeartBeat);
                    }
                }
            }
            _ => {} // no reply for follower and leader
        }
    }

    fn handle_append_entries_request(
        &mut self,
        args: AppendEntriesArgs,
    ) -> labrpc::Result<AppendEntriesReply> {
        println!("[handle_append_entries! {}] {:?}", self.me, args);
        let success = {
            if self.hard_state.current_term > args.term {
                false
            } else {
                if args.term > self.hard_state.current_term
                || matches!(self.role, RoleState::Candidate { .. }) 
                    && args.term == self.hard_state.current_term {
                    self.update_term(args.term);
                    self.turn_follower();
                }
                // TODO log replication
                // if there is no conflict, append entries
                // firstly make sure prevLogTerm and prevLogIndex match
                // if args.prev_log_index == 0 || self.hard_state.log[args.prev_log_index as usize].0 == args.prev_log_term {
                //     self.hard_state.log.truncate(args.prev_log_index as usize + 1);
                //     for entry in args.entries {
                //         self.hard_state.log.push((args.term, entry));
                //     }
                //     true
                // } else {
                //     false
                // }

                match self.role {
                    RoleState::Follower { .. } => {
                        self.schedule_event(Event::ResetTimeout);
                        true
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

    fn handle_append_entries_reply(&mut self, from: usize, reply: AppendEntriesReply) {
        println!("[handle_append_entries_reply! {}] {:?}", self.me, reply);
        if reply.term > self.hard_state.current_term {
            self.update_term(reply.term);
            self.turn_follower();
        }

        match &mut self.role {
            RoleState::Leader {
                next_index,
                match_index,
            } => {
                // TODO
            }
            _ => {} // no reply for follower and candidate
        }
    }

    fn append_entries(&mut self) {
        for (i, peer) in self.peers.iter().enumerate() {
            if i == self.me {
                continue;
            }
            let tx = self.event_loop_tx().clone();
            let fut = peer.append_entries(&self.append_entries_args());
            self.executor
                .spawn(async move {
                    if let Ok(reply) = fut.await {
                        tx.unbounded_send(Event::AppendEntriesReply(i, reply))
                            .unwrap();
                    }
                })
                .unwrap();
        }
    }
}

impl Raft {
    /// Only for suppressing deadcode warnings.
    #[doc(hidden)]
    pub fn __suppress_deadcode(&mut self) {
        let _ = self.start(&0);
        let _ = self.cond_install_snapshot(0, 0, &[]);
        let _ = self.snapshot(0, &[]);
        let _ = self.send_request_vote(0, Default::default());
        self.persist();
        let _ = &self.state;
        let _ = &self.me;
        let _ = &self.persister;
        let _ = &self.peers;
    }
}

// Choose concurrency paradigm.
//
// You can either drive the raft state machine by the rpc framework,
//
// ```rust
// struct Node { raft: Arc<Mutex<Raft>> }
// ```
//
// or spawn a new thread runs the raft state machine and communicate via
// a channel.
//
// ```rust
// struct Node { sender: Sender<Msg> }
// ```
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
                let build_rand_timeout_timer = || {
                    futures_timer::Delay::new(Duration::from_millis(
                        rand::thread_rng().gen_range(200, 500),
                    ))
                    .fuse()
                };
                let build_heartbeat_timer =
                    || futures_timer::Delay::new(Duration::from_millis(50)).fuse();

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
                            event_loop_tx.unbounded_send(Event::Timeout).unwrap();
                            timeout_timer = build_rand_timeout_timer();
                        }
                        _ = heartbeat_timer => {
                            event_loop_tx.unbounded_send(Event::HeartBeat).unwrap();
                            heartbeat_timer = build_heartbeat_timer();
                        }
                        _ = shutdown_rx => break,
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
