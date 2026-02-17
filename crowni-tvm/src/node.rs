// ═══════════════════════════════════════════════════════════════
// Crowny Distributed Node System
// 분산 노드 — 노드 ID, 피어 관리, 상태 동기화, 3진 합의
// ═══════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ── 노드 상태 ──

#[derive(Debug, Clone, PartialEq)]
pub enum NodeState {
    Follower,
    Candidate,
    Leader,
    Offline,
    Partitioned,
}

impl std::fmt::Display for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Follower => write!(f, "Follower"),
            Self::Candidate => write!(f, "Candidate"),
            Self::Leader => write!(f, "Leader"),
            Self::Offline => write!(f, "Offline"),
            Self::Partitioned => write!(f, "Partitioned"),
        }
    }
}

// ── 노드 ID ──

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId {
    pub id: String,
    pub region: String,
    pub shard: u32,
}

impl NodeId {
    pub fn new(id: &str, region: &str, shard: u32) -> Self {
        Self { id: id.to_string(), region: region.to_string(), shard }
    }

    pub fn generate(region: &str, shard: u32) -> Self {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        let id = format!("node-{}-{}-{}", region, shard, ts % 100000);
        Self { id, region: region.to_string(), shard }
    }

    pub fn short(&self) -> String {
        if self.id.len() > 12 {
            format!("{}...", &self.id[..12])
        } else {
            self.id.clone()
        }
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}:s{}", self.id, self.region, self.shard)
    }
}

// ── 피어 정보 ──

#[derive(Debug, Clone)]
pub struct Peer {
    pub node_id: NodeId,
    pub address: String,
    pub port: u16,
    pub state: NodeState,
    pub last_heartbeat: u64,
    pub term: u64,
    pub trit_state: i8,    // -1, 0, +1
    pub latency_ms: u32,
    pub synced_version: u64,
}

impl Peer {
    pub fn new(node_id: NodeId, address: &str, port: u16) -> Self {
        Self {
            node_id,
            address: address.to_string(),
            port,
            state: NodeState::Follower,
            last_heartbeat: now_ms(),
            term: 0,
            trit_state: 0,
            latency_ms: 0,
            synced_version: 0,
        }
    }

    pub fn is_alive(&self, timeout_ms: u64) -> bool {
        let elapsed = now_ms().saturating_sub(self.last_heartbeat);
        elapsed < timeout_ms
    }

    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }
}

// ── 3진 투표 ──

#[derive(Debug, Clone)]
pub struct TritVote {
    pub voter: NodeId,
    pub term: u64,
    pub vote: i8,          // -1, 0, +1
    pub reason: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct VoteResult {
    pub term: u64,
    pub total: usize,
    pub positive: usize,
    pub neutral: usize,
    pub negative: usize,
    pub consensus: i8,
    pub confidence: f64,
}

impl std::fmt::Display for VoteResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = match self.consensus {
            1 => "P(성공)",
            -1 => "T(실패)",
            _ => "O(보류)",
        };
        write!(f, "Term {} — {} | P:{} O:{} T:{} | 신뢰도:{:.0}%",
            self.term, c, self.positive, self.neutral, self.negative, self.confidence * 100.0)
    }
}

// ── 상태 동기화 메시지 ──

#[derive(Debug, Clone)]
pub enum SyncMessage {
    Heartbeat { from: NodeId, term: u64, leader_id: NodeId },
    VoteRequest { from: NodeId, term: u64, reason: String },
    VoteResponse { from: NodeId, term: u64, vote: i8 },
    StateSync { from: NodeId, version: u64, data: Vec<(String, String)> },
    StateSyncAck { from: NodeId, version: u64, accepted: bool },
    Partition { from: NodeId, detected_at: u64 },
    Rejoin { from: NodeId, last_version: u64 },
}

impl std::fmt::Display for SyncMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Heartbeat { from, term, .. } => write!(f, "♥ Heartbeat from {} (term {})", from.short(), term),
            Self::VoteRequest { from, term, reason } => write!(f, "🗳 VoteReq from {} (term {}) — {}", from.short(), term, reason),
            Self::VoteResponse { from, term, vote } => {
                let v = match vote { 1 => "P", -1 => "T", _ => "O" };
                write!(f, "📩 Vote {} from {} (term {})", v, from.short(), term)
            }
            Self::StateSync { from, version, data } => write!(f, "🔄 Sync from {} v{} ({} items)", from.short(), version, data.len()),
            Self::StateSyncAck { from, version, accepted } => write!(f, "✓ SyncAck from {} v{} ({})", from.short(), version, if *accepted { "OK" } else { "REJECT" }),
            Self::Partition { from, .. } => write!(f, "⚠ Partition detected by {}", from.short()),
            Self::Rejoin { from, last_version } => write!(f, "🔗 Rejoin {} (v{})", from.short(), last_version),
        }
    }
}

// ── 분산 노드 ──

pub struct DistributedNode {
    pub id: NodeId,
    pub state: NodeState,
    pub term: u64,
    pub voted_for: Option<NodeId>,
    pub peers: HashMap<String, Peer>,
    pub leader_id: Option<NodeId>,
    pub heartbeat_timeout_ms: u64,
    pub election_timeout_ms: u64,
    pub state_version: u64,
    pub state_data: HashMap<String, String>,
    pub message_log: Vec<SyncMessage>,
    pub vote_log: Vec<TritVote>,
}

impl DistributedNode {
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            state: NodeState::Follower,
            term: 0,
            voted_for: None,
            peers: HashMap::new(),
            leader_id: None,
            heartbeat_timeout_ms: 3000,
            election_timeout_ms: 5000,
            state_version: 0,
            state_data: HashMap::new(),
            message_log: Vec::new(),
            vote_log: Vec::new(),
        }
    }

    // ── 피어 관리 ──

    pub fn add_peer(&mut self, peer: Peer) {
        self.peers.insert(peer.node_id.id.clone(), peer);
    }

    pub fn remove_peer(&mut self, node_id: &str) -> bool {
        self.peers.remove(node_id).is_some()
    }

    pub fn alive_peers(&self) -> Vec<&Peer> {
        self.peers.values()
            .filter(|p| p.is_alive(self.heartbeat_timeout_ms))
            .collect()
    }

    pub fn dead_peers(&self) -> Vec<&Peer> {
        self.peers.values()
            .filter(|p| !p.is_alive(self.heartbeat_timeout_ms))
            .collect()
    }

    pub fn cluster_size(&self) -> usize {
        self.peers.len() + 1 // +1 for self
    }

    pub fn quorum_size(&self) -> usize {
        (self.cluster_size() / 2) + 1
    }

    // ── 하트비트 ──

    pub fn send_heartbeat(&self) -> SyncMessage {
        SyncMessage::Heartbeat {
            from: self.id.clone(),
            term: self.term,
            leader_id: self.leader_id.clone().unwrap_or_else(|| self.id.clone()),
        }
    }

    pub fn receive_heartbeat(&mut self, from: &NodeId, term: u64, leader_id: &NodeId) {
        if term >= self.term {
            self.term = term;
            self.state = NodeState::Follower;
            self.leader_id = Some(leader_id.clone());
        }
        if let Some(peer) = self.peers.get_mut(&from.id) {
            peer.last_heartbeat = now_ms();
            peer.term = term;
            peer.state = NodeState::Follower;
        }
        self.message_log.push(SyncMessage::Heartbeat {
            from: from.clone(), term, leader_id: leader_id.clone(),
        });
    }

    // ── 선거 (3진 투표) ──

    pub fn start_election(&mut self) -> SyncMessage {
        self.term += 1;
        self.state = NodeState::Candidate;
        self.voted_for = Some(self.id.clone());

        // 자기 자신에게 투표
        self.vote_log.push(TritVote {
            voter: self.id.clone(),
            term: self.term,
            vote: 1,
            reason: "self-vote".to_string(),
            timestamp: now_ms(),
        });

        SyncMessage::VoteRequest {
            from: self.id.clone(),
            term: self.term,
            reason: format!("Election for term {}", self.term),
        }
    }

    pub fn receive_vote_request(&mut self, from: &NodeId, term: u64) -> SyncMessage {
        let vote = if term > self.term && self.voted_for.is_none() {
            self.term = term;
            self.voted_for = Some(from.clone());
            1  // P: 투표
        } else if term == self.term && self.voted_for.as_ref() == Some(from) {
            1  // P: 이미 투표함
        } else if term < self.term {
            -1 // T: 구 term 거부
        } else {
            0  // O: 이미 다른 후보에 투표
        };

        self.vote_log.push(TritVote {
            voter: self.id.clone(),
            term,
            vote,
            reason: format!("vote for {} at term {}", from.short(), term),
            timestamp: now_ms(),
        });

        SyncMessage::VoteResponse {
            from: self.id.clone(),
            term,
            vote,
        }
    }

    pub fn tally_votes(&self) -> VoteResult {
        let current_votes: Vec<&TritVote> = self.vote_log.iter()
            .filter(|v| v.term == self.term)
            .collect();

        let positive = current_votes.iter().filter(|v| v.vote > 0).count();
        let neutral = current_votes.iter().filter(|v| v.vote == 0).count();
        let negative = current_votes.iter().filter(|v| v.vote < 0).count();
        let total = current_votes.len();

        let consensus = if positive > negative { 1 }
            else if negative > positive { -1 }
            else { 0 };

        let confidence = if total > 0 {
            let majority = positive.max(negative).max(neutral);
            majority as f64 / total as f64
        } else {
            0.0
        };

        VoteResult { term: self.term, total, positive, neutral, negative, consensus, confidence }
    }

    pub fn check_election_result(&mut self) -> bool {
        let result = self.tally_votes();
        if result.positive >= self.quorum_size() {
            self.state = NodeState::Leader;
            self.leader_id = Some(self.id.clone());
            true
        } else {
            false
        }
    }

    // ── 상태 동기화 ──

    pub fn set_state(&mut self, key: &str, value: &str) -> u64 {
        self.state_version += 1;
        self.state_data.insert(key.to_string(), value.to_string());
        self.state_version
    }

    pub fn get_state(&self, key: &str) -> Option<&String> {
        self.state_data.get(key)
    }

    pub fn create_sync_message(&self) -> SyncMessage {
        let data: Vec<(String, String)> = self.state_data.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        SyncMessage::StateSync {
            from: self.id.clone(),
            version: self.state_version,
            data,
        }
    }

    pub fn apply_sync(&mut self, version: u64, data: &[(String, String)]) -> SyncMessage {
        let accepted = version > self.state_version;
        if accepted {
            for (k, v) in data {
                self.state_data.insert(k.clone(), v.clone());
            }
            self.state_version = version;
        }
        SyncMessage::StateSyncAck {
            from: self.id.clone(),
            version,
            accepted,
        }
    }

    // ── 파티션 처리 ──

    pub fn detect_partition(&self) -> Vec<NodeId> {
        self.dead_peers().iter()
            .map(|p| p.node_id.clone())
            .collect()
    }

    pub fn handle_partition(&mut self) -> Option<SyncMessage> {
        let dead = self.detect_partition();
        if !dead.is_empty() {
            for d in &dead {
                if let Some(peer) = self.peers.get_mut(&d.id) {
                    peer.state = NodeState::Partitioned;
                }
            }
            Some(SyncMessage::Partition {
                from: self.id.clone(),
                detected_at: now_ms(),
            })
        } else {
            None
        }
    }

    pub fn handle_rejoin(&mut self, from: &NodeId, last_version: u64) -> SyncMessage {
        if let Some(peer) = self.peers.get_mut(&from.id) {
            peer.state = NodeState::Follower;
            peer.last_heartbeat = now_ms();
        }

        if last_version < self.state_version {
            // 뒤처진 노드에게 전체 상태 전송
            self.create_sync_message()
        } else {
            SyncMessage::StateSyncAck {
                from: self.id.clone(),
                version: self.state_version,
                accepted: true,
            }
        }
    }

    // ── 클러스터 상태 요약 ──

    pub fn cluster_summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("═══ 클러스터 상태 ═══"));
        lines.push(format!("  Self: {} [{}] term={}", self.id, self.state, self.term));
        if let Some(ref leader) = self.leader_id {
            lines.push(format!("  Leader: {}", leader));
        }
        lines.push(format!("  State Version: v{}", self.state_version));
        lines.push(format!("  Quorum: {}/{}", self.quorum_size(), self.cluster_size()));
        lines.push(format!("  Peers:"));
        for peer in self.peers.values() {
            let alive = if peer.is_alive(self.heartbeat_timeout_ms) { "●" } else { "○" };
            lines.push(format!("    {} {} [{}] latency={}ms v{}",
                alive, peer.node_id.short(), peer.state, peer.latency_ms, peer.synced_version));
        }
        lines.join("\n")
    }
}

// ── 클러스터 시뮬레이터 ──

pub struct ClusterSimulator {
    pub nodes: Vec<DistributedNode>,
}

impl ClusterSimulator {
    pub fn new(count: usize, region: &str) -> Self {
        let mut nodes = Vec::new();
        for i in 0..count {
            let id = NodeId::new(&format!("node-{}", i), region, i as u32);
            nodes.push(DistributedNode::new(id));
        }

        // 서로 피어 등록
        for i in 0..count {
            for j in 0..count {
                if i != j {
                    let peer_id = nodes[j].id.clone();
                    let peer = Peer::new(peer_id, "127.0.0.1", 7293 + j as u16);
                    nodes[i].add_peer(peer);
                }
            }
        }

        Self { nodes }
    }

    pub fn simulate_election(&mut self) -> VoteResult {
        if self.nodes.is_empty() {
            return VoteResult {
                term: 0, total: 0, positive: 0, neutral: 0, negative: 0,
                consensus: 0, confidence: 0.0,
            };
        }

        // 노드 0이 선거 시작
        let req = self.nodes[0].start_election();
        let term = self.nodes[0].term;

        // 나머지 노드들이 투표
        let mut responses = Vec::new();
        for i in 1..self.nodes.len() {
            let from = self.nodes[0].id.clone();
            let resp = self.nodes[i].receive_vote_request(&from, term);
            responses.push(resp);
        }

        // 응답 수집
        let voter_id = self.nodes[0].id.clone();
        for resp in &responses {
            if let SyncMessage::VoteResponse { vote, .. } = resp {
                self.nodes[0].vote_log.push(TritVote {
                    voter: voter_id.clone(),
                    term,
                    vote: *vote,
                    reason: "received".to_string(),
                    timestamp: now_ms(),
                });
            }
        }

        let won = self.nodes[0].check_election_result();
        if won {
            // 리더 전파
            let leader_id = self.nodes[0].id.clone();
            for i in 1..self.nodes.len() {
                self.nodes[i].leader_id = Some(leader_id.clone());
                self.nodes[i].term = term;
            }
        }

        self.nodes[0].tally_votes()
    }

    pub fn simulate_state_sync(&mut self, key: &str, value: &str) {
        if self.nodes.is_empty() { return; }

        // 리더가 상태 설정
        let version = self.nodes[0].set_state(key, value);
        let sync_msg = self.nodes[0].create_sync_message();

        // 팔로워에게 전파
        if let SyncMessage::StateSync { version, data, .. } = sync_msg {
            for i in 1..self.nodes.len() {
                self.nodes[i].apply_sync(version, &data);
            }
        }
    }

    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push("═══ Crowny 분산 클러스터 ═══".to_string());
        lines.push(format!("  총 노드: {}", self.nodes.len()));

        let leaders: Vec<_> = self.nodes.iter().filter(|n| n.state == NodeState::Leader).collect();
        let followers: Vec<_> = self.nodes.iter().filter(|n| n.state == NodeState::Follower).collect();

        lines.push(format!("  Leaders: {}  Followers: {}", leaders.len(), followers.len()));

        for node in &self.nodes {
            let role = match node.state {
                NodeState::Leader => "★ Leader",
                NodeState::Follower => "  Follow",
                NodeState::Candidate => "? Candid",
                _ => "  ------",
            };
            lines.push(format!("  {} {} term={} v{} peers={}",
                role, node.id.short(), node.term, node.state_version, node.peers.len()));
        }
        lines.join("\n")
    }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

// ═══ 데모 ═══

pub fn demo_distributed_node() {
    println!("╔═══════════════════════════════════════════╗");
    println!("║  Crowny Distributed Node System           ║");
    println!("║  분산 노드 — 3진 합의 클러스터             ║");
    println!("╚═══════════════════════════════════════════╝");
    println!();

    // 5노드 클러스터 생성
    println!("━━━ 1. 클러스터 생성 (5노드) ━━━");
    let mut cluster = ClusterSimulator::new(5, "ap-northeast-2");
    println!("{}", cluster.summary());
    println!();

    // 선거
    println!("━━━ 2. 리더 선거 (3진 투표) ━━━");
    let vote_result = cluster.simulate_election();
    println!("  {}", vote_result);
    println!("{}", cluster.summary());
    println!();

    // 상태 동기화
    println!("━━━ 3. 상태 동기화 ━━━");
    cluster.simulate_state_sync("ai.model", "claude-3.5");
    cluster.simulate_state_sync("trit.mode", "balanced");
    cluster.simulate_state_sync("consensus.quorum", "3");

    for node in &cluster.nodes {
        let model = node.get_state("ai.model").map(|s| s.as_str()).unwrap_or("없음");
        println!("  {} → ai.model={} v{}", node.id.short(), model, node.state_version);
    }
    println!();

    // 파티션 감지
    println!("━━━ 4. 파티션 시뮬레이션 ━━━");
    println!("  (노드 3,4 오프라인 시뮬레이션)");
    let dead_nodes = cluster.nodes[0].detect_partition();
    println!("  감지된 파티션: {} 노드", dead_nodes.len());
    println!("  Quorum 유지: {}", if cluster.nodes[0].alive_peers().len() + 1 >= cluster.nodes[0].quorum_size() { "✓" } else { "✗" });
    println!();

    println!("✓ 분산 노드 데모 완료 — {} 노드 클러스터", cluster.nodes.len());
}

// ═══ 테스트 ═══

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id() {
        let id = NodeId::new("node-0", "kr", 0);
        assert_eq!(id.id, "node-0");
        assert_eq!(id.region, "kr");
        assert_eq!(format!("{}", id), "node-0@kr:s0");
    }

    #[test]
    fn test_node_id_generate() {
        let id = NodeId::generate("us-east", 1);
        assert!(id.id.starts_with("node-us-east-1-"));
    }

    #[test]
    fn test_peer_alive() {
        let id = NodeId::new("p1", "kr", 0);
        let peer = Peer::new(id, "127.0.0.1", 7293);
        assert!(peer.is_alive(5000));
    }

    #[test]
    fn test_cluster_creation() {
        let cluster = ClusterSimulator::new(3, "kr");
        assert_eq!(cluster.nodes.len(), 3);
        assert_eq!(cluster.nodes[0].peers.len(), 2);
    }

    #[test]
    fn test_election() {
        let mut cluster = ClusterSimulator::new(3, "kr");
        let result = cluster.simulate_election();
        assert!(result.total > 0);
        assert_eq!(cluster.nodes[0].state, NodeState::Leader);
    }

    #[test]
    fn test_state_sync() {
        let mut cluster = ClusterSimulator::new(3, "kr");
        cluster.simulate_election();
        cluster.simulate_state_sync("key1", "value1");
        for node in &cluster.nodes {
            assert_eq!(node.get_state("key1"), Some(&"value1".to_string()));
        }
    }

    #[test]
    fn test_trit_vote_tally() {
        let mut node = DistributedNode::new(NodeId::new("n0", "kr", 0));
        node.term = 1;
        node.vote_log.push(TritVote { voter: NodeId::new("n0","kr",0), term: 1, vote: 1, reason: "".into(), timestamp: 0 });
        node.vote_log.push(TritVote { voter: NodeId::new("n1","kr",1), term: 1, vote: 1, reason: "".into(), timestamp: 0 });
        node.vote_log.push(TritVote { voter: NodeId::new("n2","kr",2), term: 1, vote: -1, reason: "".into(), timestamp: 0 });
        let result = node.tally_votes();
        assert_eq!(result.consensus, 1); // P wins
        assert_eq!(result.positive, 2);
        assert_eq!(result.negative, 1);
    }

    #[test]
    fn test_quorum() {
        let cluster = ClusterSimulator::new(5, "kr");
        assert_eq!(cluster.nodes[0].cluster_size(), 5);
        assert_eq!(cluster.nodes[0].quorum_size(), 3);
    }
}
