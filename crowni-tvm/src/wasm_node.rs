// ═══════════════════════════════════════════════════════════════
// Crowny WASM Browser Node
// 브라우저 경량 노드 — TVM 실행, P2P 합의, 상태 동기화
// ═══════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ── 브라우저 노드 타입 ──

#[derive(Debug, Clone, PartialEq)]
pub enum BrowserNodeType {
    Full,       // 전체 TVM 실행 + 합의 참여
    Light,      // 상태 조회 + 트랜잭션 제출만
    Validator,  // 합의 투표 전용
    Observer,   // 읽기 전용
}

impl std::fmt::Display for BrowserNodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "Full"),
            Self::Light => write!(f, "Light"),
            Self::Validator => write!(f, "Validator"),
            Self::Observer => write!(f, "Observer"),
        }
    }
}

// ── WASM 모듈 매니페스트 ──

#[derive(Debug, Clone)]
pub struct WasmManifest {
    pub name: String,
    pub version: String,
    pub size_bytes: u64,
    pub hash: String,
    pub modules: Vec<WasmModule>,
    pub total_opcodes: usize,
    pub trit_support: bool,
}

#[derive(Debug, Clone)]
pub struct WasmModule {
    pub name: String,
    pub exports: Vec<String>,
    pub size_bytes: u64,
    pub critical: bool,
}

impl WasmManifest {
    pub fn crowny_standard() -> Self {
        Self {
            name: "crowny-wasm".to_string(),
            version: "0.4.0".to_string(),
            size_bytes: 256_000, // ~250KB 목표
            hash: "cb33cb33".to_string(),
            modules: vec![
                WasmModule {
                    name: "tvm_core".to_string(),
                    exports: vec![
                        "tvm_init".into(), "tvm_execute".into(), "tvm_push".into(),
                        "tvm_pop".into(), "tvm_stack_top".into(), "tvm_reset".into(),
                    ],
                    size_bytes: 80_000,
                    critical: true,
                },
                WasmModule {
                    name: "trit_ops".to_string(),
                    exports: vec![
                        "trit_and".into(), "trit_or".into(), "trit_not".into(),
                        "trit_consensus".into(), "trit_from_number".into(),
                    ],
                    size_bytes: 12_000,
                    critical: true,
                },
                WasmModule {
                    name: "hanseon_compiler".to_string(),
                    exports: vec![
                        "compile".into(), "parse".into(), "tokenize".into(),
                        "emit_ir".into(), "emit_bytecode".into(),
                    ],
                    size_bytes: 60_000,
                    critical: false,
                },
                WasmModule {
                    name: "consensus_engine".to_string(),
                    exports: vec![
                        "vote".into(), "tally".into(), "propose".into(),
                        "accept_block".into(), "reject_block".into(),
                    ],
                    size_bytes: 35_000,
                    critical: true,
                },
                WasmModule {
                    name: "p2p_network".to_string(),
                    exports: vec![
                        "connect_peer".into(), "send_message".into(),
                        "broadcast".into(), "on_message".into(),
                        "peer_count".into(), "disconnect".into(),
                    ],
                    size_bytes: 40_000,
                    critical: true,
                },
                WasmModule {
                    name: "state_store".to_string(),
                    exports: vec![
                        "get".into(), "set".into(), "delete".into(),
                        "snapshot".into(), "restore".into(),
                    ],
                    size_bytes: 20_000,
                    critical: false,
                },
                WasmModule {
                    name: "crypto".to_string(),
                    exports: vec![
                        "hash_trit".into(), "sign".into(), "verify".into(),
                        "generate_keypair".into(),
                    ],
                    size_bytes: 9_000,
                    critical: true,
                },
            ],
            total_opcodes: 729,
            trit_support: true,
        }
    }
}

// ── P2P 시그널링 (WebRTC 추상화) ──

#[derive(Debug, Clone)]
pub enum P2PMessage {
    Handshake { node_id: String, node_type: BrowserNodeType, version: String },
    Heartbeat { node_id: String, timestamp: u64 },
    TritVote { node_id: String, proposal_id: u64, vote: i8 },
    StateRequest { key: String },
    StateResponse { key: String, value: String, version: u64 },
    TxSubmit { from: String, to: String, amount: u64, memo: String },
    TxConfirm { tx_id: String, trit_state: i8 },
    BlockProposal { block_id: u64, transactions: Vec<String>, proposer: String },
    BlockVote { block_id: u64, voter: String, vote: i8 },
    Sync { from_version: u64, data: Vec<(String, String)> },
}

impl std::fmt::Display for P2PMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Handshake { node_id, node_type, version } =>
                write!(f, "🤝 Handshake {} ({}) v{}", short(node_id), node_type, version),
            Self::Heartbeat { node_id, .. } =>
                write!(f, "♥ Heartbeat {}", short(node_id)),
            Self::TritVote { node_id, proposal_id, vote } => {
                let v = match vote { 1 => "P", -1 => "T", _ => "O" };
                write!(f, "🗳 Vote {} from {} on #{}", v, short(node_id), proposal_id)
            }
            Self::StateRequest { key } => write!(f, "❓ StateReq: {}", key),
            Self::StateResponse { key, value, version } => write!(f, "📦 State: {}={} (v{})", key, short(value), version),
            Self::TxSubmit { from, to, amount, .. } => write!(f, "💸 Tx: {}→{} {} CRWN", short(from), short(to), amount),
            Self::TxConfirm { tx_id, trit_state } => {
                let s = match trit_state { 1 => "P", -1 => "T", _ => "O" };
                write!(f, "✓ TxConfirm {} [{}]", short(tx_id), s)
            }
            Self::BlockProposal { block_id, transactions, proposer } =>
                write!(f, "📦 Block #{} ({} txs) by {}", block_id, transactions.len(), short(proposer)),
            Self::BlockVote { block_id, voter, vote } => {
                let v = match vote { 1 => "P", -1 => "T", _ => "O" };
                write!(f, "🗳 BlockVote #{} {} from {}", block_id, v, short(voter))
            }
            Self::Sync { from_version, data } =>
                write!(f, "🔄 Sync v{} ({} items)", from_version, data.len()),
        }
    }
}

fn short(s: &str) -> &str {
    if s.len() > 8 { &s[..8] } else { s }
}

// ── 브라우저 노드 ──

#[derive(Debug)]
pub struct BrowserNode {
    pub id: String,
    pub node_type: BrowserNodeType,
    pub connected_peers: Vec<PeerInfo>,
    pub state: HashMap<String, String>,
    pub state_version: u64,
    pub pending_votes: Vec<PendingVote>,
    pub blocks: Vec<Block>,
    pub message_log: Vec<P2PMessage>,
    pub stats: NodeStats,
}

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub id: String,
    pub node_type: BrowserNodeType,
    pub latency_ms: u32,
    pub last_seen: u64,
    pub synced: bool,
}

#[derive(Debug, Clone)]
pub struct PendingVote {
    pub proposal_id: u64,
    pub votes: Vec<(String, i8)>, // (node_id, vote)
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub id: u64,
    pub transactions: Vec<String>,
    pub proposer: String,
    pub votes: Vec<(String, i8)>,
    pub finalized: bool,
    pub trit_state: i8,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Default)]
pub struct NodeStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub votes_cast: u64,
    pub blocks_proposed: u64,
    pub blocks_finalized: u64,
    pub uptime_ms: u64,
    pub bytes_transferred: u64,
}

impl BrowserNode {
    pub fn new(id: &str, node_type: BrowserNodeType) -> Self {
        Self {
            id: id.to_string(),
            node_type,
            connected_peers: Vec::new(),
            state: HashMap::new(),
            state_version: 0,
            pending_votes: Vec::new(),
            blocks: Vec::new(),
            message_log: Vec::new(),
            stats: NodeStats::default(),
        }
    }

    // ── 피어 연결 ──

    pub fn connect(&mut self, peer_id: &str, peer_type: BrowserNodeType) -> P2PMessage {
        self.connected_peers.push(PeerInfo {
            id: peer_id.to_string(),
            node_type: peer_type,
            latency_ms: 0,
            last_seen: now_ms(),
            synced: false,
        });
        self.stats.messages_sent += 1;
        P2PMessage::Handshake {
            node_id: self.id.clone(),
            node_type: self.node_type.clone(),
            version: "0.4.0".to_string(),
        }
    }

    pub fn handle_handshake(&mut self, node_id: &str, node_type: BrowserNodeType) {
        if !self.connected_peers.iter().any(|p| p.id == node_id) {
            self.connected_peers.push(PeerInfo {
                id: node_id.to_string(),
                node_type,
                latency_ms: 0,
                last_seen: now_ms(),
                synced: false,
            });
        }
        self.stats.messages_received += 1;
    }

    pub fn peer_count(&self) -> usize {
        self.connected_peers.len()
    }

    // ── 3진 투표 ──

    pub fn propose_vote(&mut self, proposal_id: u64) -> P2PMessage {
        self.pending_votes.push(PendingVote {
            proposal_id,
            votes: vec![(self.id.clone(), 1)], // 제안자는 자동 찬성
            created_at: now_ms(),
        });
        self.stats.votes_cast += 1;
        P2PMessage::TritVote {
            node_id: self.id.clone(),
            proposal_id,
            vote: 1,
        }
    }

    pub fn cast_vote(&mut self, proposal_id: u64, vote: i8) -> P2PMessage {
        if let Some(pv) = self.pending_votes.iter_mut().find(|v| v.proposal_id == proposal_id) {
            pv.votes.push((self.id.clone(), vote));
        } else {
            self.pending_votes.push(PendingVote {
                proposal_id,
                votes: vec![(self.id.clone(), vote)],
                created_at: now_ms(),
            });
        }
        self.stats.votes_cast += 1;
        P2PMessage::TritVote {
            node_id: self.id.clone(),
            proposal_id,
            vote,
        }
    }

    pub fn receive_vote(&mut self, proposal_id: u64, voter: &str, vote: i8) {
        if let Some(pv) = self.pending_votes.iter_mut().find(|v| v.proposal_id == proposal_id) {
            if !pv.votes.iter().any(|(id, _)| id == voter) {
                pv.votes.push((voter.to_string(), vote));
            }
        }
        self.stats.messages_received += 1;
    }

    pub fn tally_vote(&self, proposal_id: u64) -> (i8, f64) {
        if let Some(pv) = self.pending_votes.iter().find(|v| v.proposal_id == proposal_id) {
            let p = pv.votes.iter().filter(|(_, v)| *v > 0).count();
            let t = pv.votes.iter().filter(|(_, v)| *v < 0).count();
            let total = pv.votes.len();
            let consensus = if p > t { 1 } else if t > p { -1 } else { 0 };
            let confidence = if total > 0 { p.max(t) as f64 / total as f64 } else { 0.0 };
            (consensus, confidence)
        } else {
            (0, 0.0)
        }
    }

    // ── 블록 제안 ──

    pub fn propose_block(&mut self, transactions: Vec<String>) -> P2PMessage {
        let block_id = self.blocks.len() as u64 + 1;
        self.blocks.push(Block {
            id: block_id,
            transactions: transactions.clone(),
            proposer: self.id.clone(),
            votes: vec![(self.id.clone(), 1)],
            finalized: false,
            trit_state: 0,
            timestamp: now_ms(),
        });
        self.stats.blocks_proposed += 1;
        P2PMessage::BlockProposal {
            block_id,
            transactions,
            proposer: self.id.clone(),
        }
    }

    pub fn vote_block(&mut self, block_id: u64, vote: i8) -> P2PMessage {
        if let Some(block) = self.blocks.iter_mut().find(|b| b.id == block_id) {
            block.votes.push((self.id.clone(), vote));
        }
        P2PMessage::BlockVote {
            block_id,
            voter: self.id.clone(),
            vote,
        }
    }

    pub fn receive_block_vote(&mut self, block_id: u64, voter: &str, vote: i8) {
        if let Some(block) = self.blocks.iter_mut().find(|b| b.id == block_id) {
            if !block.votes.iter().any(|(id, _)| id == voter) {
                block.votes.push((voter.to_string(), vote));
            }
        }
    }

    pub fn finalize_block(&mut self, block_id: u64, quorum: usize) -> bool {
        if let Some(block) = self.blocks.iter_mut().find(|b| b.id == block_id) {
            let p = block.votes.iter().filter(|(_, v)| *v > 0).count();
            let t = block.votes.iter().filter(|(_, v)| *v < 0).count();
            if p >= quorum {
                block.finalized = true;
                block.trit_state = 1;
                self.stats.blocks_finalized += 1;
                true
            } else if t >= quorum {
                block.finalized = true;
                block.trit_state = -1;
                false
            } else {
                false
            }
        } else {
            false
        }
    }

    // ── 상태 ──

    pub fn set_state(&mut self, key: &str, value: &str) {
        self.state_version += 1;
        self.state.insert(key.to_string(), value.to_string());
    }

    pub fn get_state(&self, key: &str) -> Option<&String> {
        self.state.get(key)
    }

    // ── 요약 ──

    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("  {} [{}]", self.id, self.node_type));
        lines.push(format!("    Peers: {} | State: v{} | Blocks: {}",
            self.peer_count(), self.state_version, self.blocks.len()));
        lines.push(format!("    Votes: {} | Sent: {} | Recv: {}",
            self.stats.votes_cast, self.stats.messages_sent, self.stats.messages_received));
        let finalized: Vec<_> = self.blocks.iter().filter(|b| b.finalized).collect();
        lines.push(format!("    Finalized: {}/{}", finalized.len(), self.blocks.len()));
        lines.join("\n")
    }
}

// ── 브라우저 네트워크 시뮬레이터 ──

pub struct BrowserNetwork {
    pub nodes: Vec<BrowserNode>,
}

impl BrowserNetwork {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, id: &str, node_type: BrowserNodeType) {
        self.nodes.push(BrowserNode::new(id, node_type));
    }

    pub fn connect_all(&mut self) {
        let ids: Vec<(String, BrowserNodeType)> = self.nodes.iter()
            .map(|n| (n.id.clone(), n.node_type.clone()))
            .collect();
        for i in 0..self.nodes.len() {
            for (id, nt) in &ids {
                if *id != self.nodes[i].id {
                    self.nodes[i].connect(id, nt.clone());
                }
            }
        }
    }

    pub fn simulate_consensus(&mut self, transactions: Vec<String>) -> (bool, i8) {
        if self.nodes.is_empty() { return (false, 0); }

        // 노드 0이 블록 제안
        let _proposal = self.nodes[0].propose_block(transactions.clone());
        let block_id = self.nodes[0].blocks.last().map(|b| b.id).unwrap_or(0);
        let proposer_id = self.nodes[0].id.clone();

        // 나머지 노드도 블록 기록
        for i in 1..self.nodes.len() {
            self.nodes[i].blocks.push(Block {
                id: block_id,
                transactions: transactions.clone(),
                proposer: proposer_id.clone(),
                votes: Vec::new(),
                finalized: false,
                trit_state: 0,
                timestamp: now_ms(),
            });
        }

        // 모든 노드가 투표
        let node0_id = self.nodes[0].id.clone();
        let mut all_votes: Vec<(String, i8)> = vec![(node0_id, 1)];
        for i in 1..self.nodes.len() {
            let vote = if self.nodes[i].node_type == BrowserNodeType::Observer { 0 } else { 1 };
            let _msg = self.nodes[i].vote_block(block_id, vote);
            let nid = self.nodes[i].id.clone();
            all_votes.push((nid, vote));
        }

        // 투표 수집
        let votes_clone = all_votes.clone();
        for (voter, vote) in &votes_clone {
            for i in 0..self.nodes.len() {
                if self.nodes[i].id != *voter {
                    self.nodes[i].receive_block_vote(block_id, voter, *vote);
                }
            }
        }

        // 합의 확인
        let quorum = (self.nodes.len() / 2) + 1;
        let finalized = self.nodes[0].finalize_block(block_id, quorum);
        let state = if finalized { 1 } else { 0 };

        // 결과 전파
        if finalized {
            for i in 1..self.nodes.len() {
                self.nodes[i].finalize_block(block_id, quorum);
            }
        }

        (finalized, state)
    }

    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push("═══ Crowny 브라우저 P2P 네트워크 ═══".to_string());
        lines.push(format!("  총 노드: {}", self.nodes.len()));
        let full = self.nodes.iter().filter(|n| n.node_type == BrowserNodeType::Full).count();
        let light = self.nodes.iter().filter(|n| n.node_type == BrowserNodeType::Light).count();
        let val = self.nodes.iter().filter(|n| n.node_type == BrowserNodeType::Validator).count();
        let obs = self.nodes.iter().filter(|n| n.node_type == BrowserNodeType::Observer).count();
        lines.push(format!("  Full: {} | Light: {} | Validator: {} | Observer: {}", full, light, val, obs));
        for node in &self.nodes {
            lines.push(node.summary());
        }
        lines.join("\n")
    }
}

// ── JS 바인딩 생성기 ──

pub fn generate_js_bindings() -> String {
    let mut js = String::new();
    js.push_str("// ═══ Crowny WASM Browser Node ═══\n");
    js.push_str("// Auto-generated JS bindings for TVM WASM\n\n");

    js.push_str("class CrownyWasmNode {\n");
    js.push_str("  constructor() {\n");
    js.push_str("    this.wasm = null;\n");
    js.push_str("    this.nodeId = crypto.randomUUID();\n");
    js.push_str("    this.peers = new Map();\n");
    js.push_str("    this.state = new Map();\n");
    js.push_str("    this.blocks = [];\n");
    js.push_str("  }\n\n");

    js.push_str("  async init(wasmUrl = '/crowny-tvm.wasm') {\n");
    js.push_str("    const response = await fetch(wasmUrl);\n");
    js.push_str("    const bytes = await response.arrayBuffer();\n");
    js.push_str("    const { instance } = await WebAssembly.instantiate(bytes, {\n");
    js.push_str("      env: {\n");
    js.push_str("        print: (ptr, len) => console.log(this._readString(ptr, len)),\n");
    js.push_str("        now_ms: () => BigInt(Date.now()),\n");
    js.push_str("      }\n");
    js.push_str("    });\n");
    js.push_str("    this.wasm = instance.exports;\n");
    js.push_str("    this.wasm.tvm_init();\n");
    js.push_str("    return this;\n");
    js.push_str("  }\n\n");

    js.push_str("  // TVM 실행\n");
    js.push_str("  execute(source) { return this.wasm.tvm_execute(source); }\n");
    js.push_str("  push(value) { this.wasm.tvm_push(value); }\n");
    js.push_str("  pop() { return this.wasm.tvm_pop(); }\n");
    js.push_str("  stackTop() { return this.wasm.tvm_stack_top(); }\n\n");

    js.push_str("  // Trit 연산\n");
    js.push_str("  tritAnd(a, b) { return this.wasm.trit_and(a, b); }\n");
    js.push_str("  tritOr(a, b) { return this.wasm.trit_or(a, b); }\n");
    js.push_str("  tritNot(a) { return this.wasm.trit_not(a); }\n\n");

    js.push_str("  // P2P (WebRTC)\n");
    js.push_str("  async connectPeer(peerId, signalingUrl) {\n");
    js.push_str("    const pc = new RTCPeerConnection({ iceServers: [{ urls: 'stun:stun.l.google.com:19302' }] });\n");
    js.push_str("    const dc = pc.createDataChannel('crowny');\n");
    js.push_str("    dc.onmessage = (e) => this._handleMessage(peerId, JSON.parse(e.data));\n");
    js.push_str("    this.peers.set(peerId, { pc, dc });\n");
    js.push_str("    const offer = await pc.createOffer();\n");
    js.push_str("    await pc.setLocalDescription(offer);\n");
    js.push_str("    return offer;\n");
    js.push_str("  }\n\n");

    js.push_str("  broadcast(message) {\n");
    js.push_str("    const data = JSON.stringify(message);\n");
    js.push_str("    for (const [id, { dc }] of this.peers) {\n");
    js.push_str("      if (dc.readyState === 'open') dc.send(data);\n");
    js.push_str("    }\n");
    js.push_str("  }\n\n");

    js.push_str("  // 3진 투표\n");
    js.push_str("  vote(proposalId, vote) {\n");
    js.push_str("    this.broadcast({ type: 'vote', nodeId: this.nodeId, proposalId, vote });\n");
    js.push_str("  }\n\n");

    js.push_str("  // 합의\n");
    js.push_str("  consensus(votes) {\n");
    js.push_str("    const p = votes.filter(v => v > 0).length;\n");
    js.push_str("    const t = votes.filter(v => v < 0).length;\n");
    js.push_str("    return p > t ? 1 : t > p ? -1 : 0;\n");
    js.push_str("  }\n\n");

    js.push_str("  _handleMessage(peerId, msg) {\n");
    js.push_str("    switch(msg.type) {\n");
    js.push_str("      case 'vote': this._onVote(msg); break;\n");
    js.push_str("      case 'block': this._onBlock(msg); break;\n");
    js.push_str("      case 'sync': this._onSync(msg); break;\n");
    js.push_str("      case 'heartbeat': this._onHeartbeat(peerId); break;\n");
    js.push_str("    }\n");
    js.push_str("  }\n\n");

    js.push_str("  _onVote(msg) { /* 투표 처리 */ }\n");
    js.push_str("  _onBlock(msg) { /* 블록 처리 */ }\n");
    js.push_str("  _onSync(msg) { /* 동기화 처리 */ }\n");
    js.push_str("  _onHeartbeat(peerId) { /* 하트비트 처리 */ }\n");
    js.push_str("}\n\n");

    js.push_str("// Quick start\n");
    js.push_str("// const node = await new CrownyWasmNode().init();\n");
    js.push_str("// node.push(42); node.push(58); node.execute('add');\n");
    js.push_str("// console.log(node.stackTop()); // 100\n");
    js.push_str("export default CrownyWasmNode;\n");

    js
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

// ═══ 데모 ═══

pub fn demo_wasm_browser_node() {
    println!("╔═══════════════════════════════════════════╗");
    println!("║  Crowny WASM Browser Node                 ║");
    println!("║  브라우저 경량 노드 — P2P 합의 네트워크     ║");
    println!("╚═══════════════════════════════════════════╝");
    println!();

    // 1. WASM 매니페스트
    println!("━━━ 1. WASM 모듈 매니페스트 ━━━");
    let manifest = WasmManifest::crowny_standard();
    println!("  {} v{} ({:.1}KB)", manifest.name, manifest.version, manifest.size_bytes as f64 / 1024.0);
    println!("  Opcodes: {} | Trit: {}", manifest.total_opcodes, manifest.trit_support);
    for m in &manifest.modules {
        let critical = if m.critical { "●" } else { "○" };
        println!("    {} {} ({:.1}KB) — {} exports",
            critical, m.name, m.size_bytes as f64 / 1024.0, m.exports.len());
    }
    println!();

    // 2. 브라우저 네트워크
    println!("━━━ 2. 브라우저 P2P 네트워크 ━━━");
    let mut network = BrowserNetwork::new();
    network.add_node("browser-seoul-1", BrowserNodeType::Full);
    network.add_node("browser-tokyo-2", BrowserNodeType::Full);
    network.add_node("browser-sf-3", BrowserNodeType::Validator);
    network.add_node("browser-london-4", BrowserNodeType::Light);
    network.add_node("browser-berlin-5", BrowserNodeType::Observer);
    network.connect_all();
    println!("{}", network.summary());
    println!();

    // 3. 블록 합의
    println!("━━━ 3. 브라우저 블록 합의 ━━━");
    let txs = vec![
        "tx-001: alice→bob 100 CRWN".to_string(),
        "tx-002: bob→carol 50 CRWN".to_string(),
        "tx-003: carol→dave 25 CRWN".to_string(),
    ];
    let (finalized, state) = network.simulate_consensus(txs);
    let state_str = match state { 1 => "P(확정)", -1 => "T(거부)", _ => "O(보류)" };
    println!("  블록 #1: {} | 합의: {}", if finalized { "확정" } else { "미확정" }, state_str);

    // 투표 상세
    if let Some(block) = network.nodes[0].blocks.first() {
        for (voter, vote) in &block.votes {
            let v = match vote { 1 => "P", -1 => "T", _ => "O" };
            println!("    {} → {}", short(voter), v);
        }
    }
    println!();

    // 4. 연속 블록
    println!("━━━ 4. 연속 블록 생성 ━━━");
    for i in 2..=5 {
        let txs = vec![format!("tx-batch-{}", i)];
        let (fin, st) = network.simulate_consensus(txs);
        let s = match st { 1 => "P", -1 => "T", _ => "O" };
        println!("  Block #{}: {} [{}]", i, if fin { "✓" } else { "○" }, s);
    }
    println!();

    // 5. JS 바인딩
    println!("━━━ 5. JS 바인딩 생성 ━━━");
    let js = generate_js_bindings();
    let js_lines = js.lines().count();
    println!("  Generated: {} lines JavaScript", js_lines);
    println!("  Class: CrownyWasmNode");
    println!("  Methods: init, execute, push, pop, connectPeer, broadcast, vote, consensus");
    println!();

    // 6. 최종 상태
    println!("━━━ 6. 네트워크 최종 상태 ━━━");
    println!("{}", network.summary());
    println!();

    println!("✓ WASM 브라우저 노드 데모 완료 — {} 노드, {} 블록",
        network.nodes.len(),
        network.nodes[0].blocks.len());
}

// ═══ 테스트 ═══

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_node_creation() {
        let node = BrowserNode::new("test-1", BrowserNodeType::Full);
        assert_eq!(node.id, "test-1");
        assert_eq!(node.node_type, BrowserNodeType::Full);
        assert_eq!(node.peer_count(), 0);
    }

    #[test]
    fn test_peer_connection() {
        let mut node = BrowserNode::new("n1", BrowserNodeType::Full);
        node.connect("n2", BrowserNodeType::Light);
        assert_eq!(node.peer_count(), 1);
    }

    #[test]
    fn test_vote_tally() {
        let mut node = BrowserNode::new("n1", BrowserNodeType::Full);
        node.propose_vote(1);
        node.receive_vote(1, "n2", 1);
        node.receive_vote(1, "n3", -1);
        let (consensus, confidence) = node.tally_vote(1);
        assert_eq!(consensus, 1); // 2P vs 1T
    }

    #[test]
    fn test_block_proposal() {
        let mut node = BrowserNode::new("n1", BrowserNodeType::Full);
        node.propose_block(vec!["tx1".into(), "tx2".into()]);
        assert_eq!(node.blocks.len(), 1);
        assert_eq!(node.blocks[0].transactions.len(), 2);
    }

    #[test]
    fn test_block_finalization() {
        let mut node = BrowserNode::new("n1", BrowserNodeType::Full);
        node.propose_block(vec!["tx1".into()]);
        node.receive_block_vote(1, "n2", 1);
        node.receive_block_vote(1, "n3", 1);
        let finalized = node.finalize_block(1, 2);
        assert!(finalized);
        assert_eq!(node.blocks[0].trit_state, 1);
    }

    #[test]
    fn test_network_consensus() {
        let mut net = BrowserNetwork::new();
        net.add_node("n1", BrowserNodeType::Full);
        net.add_node("n2", BrowserNodeType::Full);
        net.add_node("n3", BrowserNodeType::Validator);
        net.connect_all();
        let (fin, state) = net.simulate_consensus(vec!["tx".into()]);
        assert!(fin);
        assert_eq!(state, 1);
    }

    #[test]
    fn test_wasm_manifest() {
        let m = WasmManifest::crowny_standard();
        assert_eq!(m.total_opcodes, 729);
        assert!(m.trit_support);
        assert!(m.modules.len() >= 5);
    }

    #[test]
    fn test_js_bindings() {
        let js = generate_js_bindings();
        assert!(js.contains("CrownyWasmNode"));
        assert!(js.contains("tvm_init"));
        assert!(js.contains("WebRTC"));
    }

    #[test]
    fn test_state_management() {
        let mut node = BrowserNode::new("n1", BrowserNodeType::Full);
        node.set_state("key1", "val1");
        assert_eq!(node.get_state("key1"), Some(&"val1".to_string()));
        assert_eq!(node.state_version, 1);
    }
}
