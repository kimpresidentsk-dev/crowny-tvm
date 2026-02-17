// ═══════════════════════════════════════════════════════════════
// Crowny Chain — 3진 블록체인
// PoT(Proof of Trit) 합의 · 블록 생성/검증 · 체인 연결
// 트랜잭션 풀 · 머클트리 · 밸리데이터 네트워크
// ═══════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64 }

// ═══════════════════════════════════════
// 해시 (3진 해시)
// ═══════════════════════════════════════

pub fn trit_hash(data: &str) -> String {
    let mut h: u64 = 0xcb735a4e9f1d2b08;
    for (i, b) in data.bytes().enumerate() {
        h ^= (b as u64).wrapping_mul(0x100000001b3);
        h = h.wrapping_mul(0x517cc1b727220a95);
        h ^= (i as u64).wrapping_add(0x9e3779b97f4a7c15);
        h = h.rotate_left(17) ^ h.rotate_right(23);
    }
    // 64비트 → 27-trit 표현
    let trits: String = (0..27).map(|i| {
        match ((h >> (i * 2)) & 3) % 3 {
            0 => 'P', 1 => 'O', _ => 'T',
        }
    }).collect();
    format!("0t{}", trits)
}

pub fn trit_hash_bytes(data: &[u8]) -> String {
    let s: String = data.iter().map(|b| format!("{:02x}", b)).collect();
    trit_hash(&s)
}

// ═══════════════════════════════════════
// 트랜잭션
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: String,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub fee: u64,
    pub data: String,          // 임의 데이터 (스마트 컨트랙트 호출 등)
    pub trit_type: TxType,
    pub signature: String,
    pub timestamp: u64,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TxType {
    Transfer,       // P: CRWN 전송
    Stake,          // P: 스테이킹
    Unstake,        // T: 언스테이킹
    Vote,           // O: 합의 투표
    ContractDeploy, // P: 컨트랙트 배포
    ContractCall,   // O: 컨트랙트 호출
    Reward,         // P: 블록 보상
}

impl std::fmt::Display for TxType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transfer => write!(f, "전송"),
            Self::Stake => write!(f, "스테이킹"),
            Self::Unstake => write!(f, "언스테이킹"),
            Self::Vote => write!(f, "투표"),
            Self::ContractDeploy => write!(f, "컨트랙트배포"),
            Self::ContractCall => write!(f, "컨트랙트호출"),
            Self::Reward => write!(f, "블록보상"),
        }
    }
}

impl Transaction {
    pub fn new(from: &str, to: &str, amount: u64, fee: u64, tx_type: TxType, data: &str) -> Self {
        let ts = now_ms();
        let raw = format!("{}:{}:{}:{}:{}", from, to, amount, ts, data);
        let hash = trit_hash(&raw);
        let sig = trit_hash(&format!("sig:{}", raw));
        Self {
            id: hash.clone(), from: from.into(), to: to.into(),
            amount, fee, data: data.into(), trit_type: tx_type,
            signature: sig, timestamp: ts, hash,
        }
    }

    pub fn verify(&self) -> bool {
        let raw = format!("{}:{}:{}:{}:{}", self.from, self.to, self.amount, self.timestamp, self.data);
        let expected = trit_hash(&raw);
        self.hash == expected
    }

    pub fn trit(&self) -> i8 {
        match &self.trit_type {
            TxType::Transfer | TxType::Stake | TxType::ContractDeploy | TxType::Reward => 1,
            TxType::Vote | TxType::ContractCall => 0,
            TxType::Unstake => -1,
        }
    }
}

impl std::fmt::Display for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let trit = match self.trit() { 1 => "P", -1 => "T", _ => "O" };
        write!(f, "[{}] {} {}→{} {} CRWN (fee:{}) {:.8}",
            trit, self.trit_type, self.from, self.to, self.amount, self.fee, self.hash)
    }
}

// ═══════════════════════════════════════
// 머클트리 (3진)
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct MerkleNode {
    pub hash: String,
    pub left: Option<Box<MerkleNode>>,
    pub middle: Option<Box<MerkleNode>>,  // 3진: 3방향 머클
    pub right: Option<Box<MerkleNode>>,
}

impl MerkleNode {
    pub fn leaf(data: &str) -> Self {
        Self { hash: trit_hash(data), left: None, middle: None, right: None }
    }

    pub fn branch(children: Vec<MerkleNode>) -> Self {
        let combined: String = children.iter().map(|c| c.hash.clone()).collect();
        let hash = trit_hash(&combined);
        let mut iter = children.into_iter();
        Self {
            hash,
            left: iter.next().map(Box::new),
            middle: iter.next().map(Box::new),
            right: iter.next().map(Box::new),
        }
    }
}

pub fn build_merkle_root(tx_hashes: &[String]) -> String {
    if tx_hashes.is_empty() { return trit_hash("empty"); }
    if tx_hashes.len() == 1 { return tx_hashes[0].clone(); }

    let mut nodes: Vec<String> = tx_hashes.to_vec();
    while nodes.len() > 1 {
        let mut next = Vec::new();
        // 3개씩 묶어서 해시 (3진 머클)
        for chunk in nodes.chunks(3) {
            let combined: String = chunk.iter().cloned().collect();
            next.push(trit_hash(&combined));
        }
        nodes = next;
    }
    nodes[0].clone()
}

// ═══════════════════════════════════════
// 블록
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub prev_hash: String,
    pub hash: String,
    pub merkle_root: String,
    pub transactions: Vec<Transaction>,
    pub validator: String,
    pub pot_proof: PoTProof,       // Proof of Trit
    pub trit_state: i8,
    pub ctp_header: [i8; 9],
    pub tx_count: usize,
    pub total_fees: u64,
    pub block_reward: u64,
}

impl Block {
    pub fn new(index: u64, prev_hash: &str, txs: Vec<Transaction>, validator: &str, proof: PoTProof) -> Self {
        let tx_hashes: Vec<String> = txs.iter().map(|t| t.hash.clone()).collect();
        let merkle_root = build_merkle_root(&tx_hashes);
        let total_fees: u64 = txs.iter().map(|t| t.fee).sum();
        let block_reward = 100; // 블록당 100 CRWN
        let tx_count = txs.len();
        let ts = now_ms();

        let raw = format!("{}:{}:{}:{}:{}", index, prev_hash, merkle_root, validator, ts);
        let hash = trit_hash(&raw);

        // CTP 헤더 생성
        let consensus_trit = proof.consensus_trit();
        let mut ctp = [0i8; 9];
        ctp[0] = consensus_trit;
        ctp[1] = 1; // permission
        ctp[2] = if proof.unanimous() { 1 } else { 0 };
        ctp[3] = if proof.votes.len() >= 2 { 1 } else { 0 };
        ctp[4] = 1; // routing
        for (i, v) in proof.votes.iter().take(4).enumerate() {
            ctp[5 + i] = v.trit;
        }

        Self {
            index, timestamp: ts, prev_hash: prev_hash.into(),
            hash, merkle_root, transactions: txs, validator: validator.into(),
            pot_proof: proof, trit_state: consensus_trit,
            ctp_header: ctp, tx_count, total_fees, block_reward,
        }
    }

    pub fn genesis() -> Self {
        let genesis_tx = Transaction::new("genesis", "treasury", 153_000_000, 0, TxType::Reward, "Genesis Block");
        let proof = PoTProof {
            round: 0,
            votes: vec![
                ValidatorVote { validator: "genesis".into(), trit: 1, reason: "창세 블록".into(), timestamp: now_ms() },
            ],
            threshold: 1,
        };
        Block::new(0, "0t000000000000000000000000000", vec![genesis_tx], "genesis", proof)
    }

    pub fn verify(&self) -> bool {
        // 1. 해시 검증
        let tx_hashes: Vec<String> = self.transactions.iter().map(|t| t.hash.clone()).collect();
        let merkle = build_merkle_root(&tx_hashes);
        if merkle != self.merkle_root { return false; }

        // 2. 트랜잭션 검증
        for tx in &self.transactions {
            if !tx.verify() { return false; }
        }

        // 3. PoT 검증
        if !self.pot_proof.is_valid() { return false; }

        true
    }

    pub fn ctp_string(&self) -> String {
        self.ctp_header.iter().map(|t| match t { 1 => 'P', -1 => 'T', _ => 'O' }).collect()
    }
}

impl std::fmt::Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let trit = match self.trit_state { 1 => "P", -1 => "T", _ => "O" };
        write!(f, "Block #{} [{}] — {} txs | fee:{} | reward:{} | CTP:{} | {:.12}",
            self.index, trit, self.tx_count, self.total_fees, self.block_reward,
            self.ctp_string(), self.hash)
    }
}

// ═══════════════════════════════════════
// PoT (Proof of Trit) 합의
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct ValidatorVote {
    pub validator: String,
    pub trit: i8,       // P=1, O=0, T=-1
    pub reason: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct PoTProof {
    pub round: u64,
    pub votes: Vec<ValidatorVote>,
    pub threshold: usize,   // 최소 투표 수
}

impl PoTProof {
    pub fn new(round: u64, threshold: usize) -> Self {
        Self { round, votes: Vec::new(), threshold }
    }

    pub fn add_vote(&mut self, validator: &str, trit: i8, reason: &str) {
        self.votes.push(ValidatorVote {
            validator: validator.into(), trit, reason: reason.into(), timestamp: now_ms(),
        });
    }

    pub fn consensus_trit(&self) -> i8 {
        let p = self.votes.iter().filter(|v| v.trit > 0).count();
        let t = self.votes.iter().filter(|v| v.trit < 0).count();
        if p > t { 1 } else if t > p { -1 } else { 0 }
    }

    pub fn unanimous(&self) -> bool {
        if self.votes.is_empty() { return false; }
        let first = self.votes[0].trit;
        self.votes.iter().all(|v| v.trit == first)
    }

    pub fn is_valid(&self) -> bool {
        self.votes.len() >= self.threshold && self.consensus_trit() >= 0
    }

    pub fn confidence(&self) -> f64 {
        if self.votes.is_empty() { return 0.0; }
        let con = self.consensus_trit();
        let agree = self.votes.iter().filter(|v| v.trit == con).count();
        agree as f64 / self.votes.len() as f64
    }
}

// ═══════════════════════════════════════
// 밸리데이터
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Validator {
    pub address: String,
    pub name: String,
    pub stake: u64,
    pub blocks_produced: u64,
    pub blocks_missed: u64,
    pub reputation: f64,        // 0.0 ~ 1.0
    pub active: bool,
    pub joined_at: u64,
}

impl Validator {
    pub fn new(address: &str, name: &str, stake: u64) -> Self {
        Self {
            address: address.into(), name: name.into(), stake,
            blocks_produced: 0, blocks_missed: 0,
            reputation: 1.0, active: true, joined_at: now_ms(),
        }
    }

    pub fn trit(&self) -> i8 {
        if self.reputation > 0.7 && self.active { 1 }
        else if self.reputation > 0.3 { 0 }
        else { -1 }
    }
}

impl std::fmt::Display for Validator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let trit = match self.trit() { 1 => "P", -1 => "T", _ => "O" };
        let status = if self.active { "●" } else { "○" };
        write!(f, "[{}]{} {} — {} CRWN staked | {} blocks | rep:{:.2}",
            trit, status, self.name, self.stake, self.blocks_produced, self.reputation)
    }
}

// ═══════════════════════════════════════
// 트랜잭션 풀
// ═══════════════════════════════════════

pub struct TxPool {
    pub pending: Vec<Transaction>,
    pub max_size: usize,
}

impl TxPool {
    pub fn new(max_size: usize) -> Self { Self { pending: Vec::new(), max_size } }

    pub fn add(&mut self, tx: Transaction) -> bool {
        if self.pending.len() >= self.max_size { return false; }
        if !tx.verify() { return false; }
        self.pending.push(tx);
        true
    }

    pub fn take_batch(&mut self, max_txs: usize) -> Vec<Transaction> {
        // 수수료 높은 순으로 정렬 후 추출
        self.pending.sort_by(|a, b| b.fee.cmp(&a.fee));
        let batch: Vec<Transaction> = self.pending.drain(..self.pending.len().min(max_txs)).collect();
        batch
    }

    pub fn size(&self) -> usize { self.pending.len() }
}

// ═══════════════════════════════════════
// 블록체인
// ═══════════════════════════════════════

pub struct CrownyChain {
    pub blocks: Vec<Block>,
    pub validators: Vec<Validator>,
    pub tx_pool: TxPool,
    pub balances: HashMap<String, u64>,
    pub stakes: HashMap<String, u64>,
    pub chain_id: String,
    pub block_time_ms: u64,
    pub max_block_txs: usize,
}

impl CrownyChain {
    pub fn new() -> Self {
        let genesis = Block::genesis();
        let mut balances = HashMap::new();
        balances.insert("treasury".into(), 153_000_000);

        Self {
            blocks: vec![genesis],
            validators: Vec::new(),
            tx_pool: TxPool::new(10000),
            balances,
            stakes: HashMap::new(),
            chain_id: "crowny-mainnet-1".into(),
            block_time_ms: 3000, // 3초 블록타임
            max_block_txs: 100,
        }
    }

    pub fn add_validator(&mut self, address: &str, name: &str, stake: u64) -> bool {
        let bal = self.balances.get(address).copied().unwrap_or(0);
        if bal < stake { return false; }
        *self.balances.entry(address.into()).or_insert(0) -= stake;
        *self.stakes.entry(address.into()).or_insert(0) += stake;
        self.validators.push(Validator::new(address, name, stake));
        true
    }

    pub fn submit_tx(&mut self, tx: Transaction) -> bool {
        self.tx_pool.add(tx)
    }

    pub fn transfer(&mut self, from: &str, to: &str, amount: u64, fee: u64) -> bool {
        let bal = self.balances.get(from).copied().unwrap_or(0);
        if bal < amount + fee { return false; }
        let tx = Transaction::new(from, to, amount, fee, TxType::Transfer, "");
        self.tx_pool.add(tx)
    }

    pub fn select_validator(&self) -> Option<&Validator> {
        // 스테이크 가중 선택 (시뮬레이션: 최고 스테이크)
        self.validators.iter()
            .filter(|v| v.active && v.reputation > 0.3)
            .max_by_key(|v| v.stake)
    }

    pub fn produce_block(&mut self) -> Option<Block> {
        let validator = match self.select_validator() {
            Some(v) => v.name.clone(),
            None => return None,
        };

        // TX 배치 추출
        let mut txs = self.tx_pool.take_batch(self.max_block_txs);
        if txs.is_empty() { return None; }

        // PoT 합의 투표
        let mut proof = PoTProof::new(self.blocks.len() as u64, 2);
        for v in &self.validators {
            if !v.active { continue; }
            let trit = if v.reputation > 0.5 { 1 } else { 0 };
            proof.add_vote(&v.name, trit, &format!("검증 완료 (rep:{:.2})", v.reputation));
        }

        if !proof.is_valid() { return None; }

        // 블록 보상 TX
        let reward_tx = Transaction::new("network", &validator, 100, 0, TxType::Reward, "block reward");
        txs.push(reward_tx);

        let prev_hash = self.blocks.last().map(|b| b.hash.clone()).unwrap_or_default();
        let block = Block::new(self.blocks.len() as u64, &prev_hash, txs, &validator, proof);

        // 잔액 업데이트
        for tx in &block.transactions {
            if tx.trit_type == TxType::Reward {
                *self.balances.entry(tx.to.clone()).or_insert(0) += tx.amount;
            } else {
                let from_bal = self.balances.entry(tx.from.clone()).or_insert(0);
                *from_bal = from_bal.saturating_sub(tx.amount + tx.fee);
                *self.balances.entry(tx.to.clone()).or_insert(0) += tx.amount;
            }
        }

        // 밸리데이터 통계
        if let Some(v) = self.validators.iter_mut().find(|v| v.name == validator) {
            v.blocks_produced += 1;
        }

        self.blocks.push(block.clone());
        Some(block)
    }

    pub fn verify_chain(&self) -> (bool, usize) {
        let mut valid = 0;
        for i in 1..self.blocks.len() {
            let block = &self.blocks[i];
            let prev = &self.blocks[i - 1];
            if block.prev_hash != prev.hash { return (false, i); }
            if !block.verify() { return (false, i); }
            valid += 1;
        }
        (true, valid)
    }

    pub fn height(&self) -> u64 { self.blocks.len() as u64 - 1 }

    pub fn latest(&self) -> Option<&Block> { self.blocks.last() }

    pub fn balance_of(&self, address: &str) -> u64 {
        self.balances.get(address).copied().unwrap_or(0)
    }

    pub fn summary(&self) -> String {
        let (valid, count) = self.verify_chain();
        let total_txs: usize = self.blocks.iter().map(|b| b.tx_count).sum();
        let total_fees: u64 = self.blocks.iter().map(|b| b.total_fees).sum();
        format!(
            "CrownyChain [{}]\n  높이: {} | 블록: {} | TX: {} | 검증: {}/{}\n  밸리데이터: {} | TX풀: {} | 총 수수료: {} CRWN",
            self.chain_id, self.height(), self.blocks.len(), total_txs,
            if valid { "✓" } else { "✗" }, count,
            self.validators.len(), self.tx_pool.size(), total_fees
        )
    }
}

// ═══ 데모 ═══

pub fn demo_chain() {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║  Crowny Chain — 3진 블록체인                    ║");
    println!("║  PoT 합의 · 블록 생성/검증 · 체인 연결           ║");
    println!("╚═══════════════════════════════════════════════╝");
    println!();

    let mut chain = CrownyChain::new();

    // 1. 제네시스
    println!("━━━ 1. 제네시스 블록 ━━━");
    let genesis = &chain.blocks[0];
    println!("  {}", genesis);
    println!("  머클: {:.20}...", genesis.merkle_root);
    println!("  Treasury: {} CRWN", chain.balance_of("treasury"));
    println!();

    // 2. 토큰 분배
    println!("━━━ 2. 초기 분배 ━━━");
    let distributions = vec![
        ("alice", 1_000_000), ("bob", 500_000), ("carol", 300_000),
        ("dave", 200_000), ("eve", 150_000),
    ];
    for (addr, amount) in &distributions {
        let bal = chain.balances.get_mut("treasury").unwrap();
        *bal -= amount;
        chain.balances.insert(addr.to_string(), *amount);
        println!("  [P] treasury → {} : {} CRWN", addr, amount);
    }
    println!("  Treasury 잔액: {} CRWN", chain.balance_of("treasury"));
    println!();

    // 3. 밸리데이터 등록
    println!("━━━ 3. 밸리데이터 등록 ━━━");
    chain.add_validator("alice", "Alice-Node", 100_000);
    chain.add_validator("bob", "Bob-Node", 80_000);
    chain.add_validator("carol", "Carol-Node", 50_000);
    for v in &chain.validators {
        println!("  {}", v);
    }
    println!();

    // 4. 트랜잭션 제출
    println!("━━━ 4. 트랜잭션 제출 ━━━");
    let txs = vec![
        ("alice", "bob", 10_000, 10, "서비스 대금"),
        ("bob", "carol", 5_000, 5, "합의 보수"),
        ("carol", "dave", 2_000, 3, "NFT 구매"),
        ("alice", "eve", 15_000, 10, "플랫폼 수수료"),
        ("dave", "alice", 1_000, 2, "리펀드"),
        ("eve", "bob", 3_000, 5, "스테이킹 대행"),
        ("alice", "carol", 8_000, 8, "거버넌스 보상"),
        ("bob", "dave", 4_000, 4, "개발 용역"),
    ];
    for (from, to, amount, fee, memo) in &txs {
        let tx = Transaction::new(from, to, *amount, *fee, TxType::Transfer, memo);
        println!("  {}", tx);
        chain.submit_tx(tx);
    }
    println!("  TX풀: {} pending", chain.tx_pool.size());
    println!();

    // 5. 블록 생성
    println!("━━━ 5. 블록 생성 (PoT 합의) ━━━");
    for round in 0..3 {
        // 추가 TX
        if round > 0 {
            chain.transfer(&format!("alice"), "bob", 1000 * (round + 1) as u64, 5);
            chain.transfer("bob", "carol", 500 * (round + 1) as u64, 3);
        }

        if let Some(block) = chain.produce_block() {
            println!("  ┌─ {}", block);
            println!("  │  밸리데이터: {} | 머클: {:.20}...", block.validator, block.merkle_root);
            println!("  │  PoT: {} 투표 (신뢰도 {:.0}%)", block.pot_proof.votes.len(), block.pot_proof.confidence() * 100.0);
            for vote in &block.pot_proof.votes {
                let trit = match vote.trit { 1 => "P", -1 => "T", _ => "O" };
                println!("  │    [{}] {} — {}", trit, vote.validator, vote.reason);
            }
            println!("  │  prev: {:.20}...", block.prev_hash);
            println!("  └─ hash: {:.20}...", block.hash);
            println!();
        }
    }

    // 6. 체인 검증
    println!("━━━ 6. 체인 검증 ━━━");
    let (valid, count) = chain.verify_chain();
    println!("  체인 무결성: {} ({} 블록 검증)", if valid { "✓ 유효" } else { "✗ 무효" }, count);
    for (i, block) in chain.blocks.iter().enumerate() {
        let trit = match block.trit_state { 1 => "P", -1 => "T", _ => "O" };
        let verified = if i == 0 { true } else { block.verify() };
        let v = if verified { "✓" } else { "✗" };
        println!("  {} #{} [{}] {} tx | {:.16}.. → {:.16}..",
            v, block.index, trit, block.tx_count,
            block.prev_hash, block.hash);
    }
    println!();

    // 7. 잔액 확인
    println!("━━━ 7. 최종 잔액 ━━━");
    let accounts = vec!["treasury", "alice", "bob", "carol", "dave", "eve"];
    for addr in &accounts {
        let bal = chain.balance_of(addr);
        let staked = chain.stakes.get(*addr).copied().unwrap_or(0);
        println!("  {:<10} {:>12} CRWN  (staked: {})", addr, bal, staked);
    }
    println!();

    // 8. 체인 요약
    println!("━━━ 8. 체인 요약 ━━━");
    println!("{}", chain.summary());
    println!();
    println!("✓ Crowny Chain 데모 완료");
}

// ═══ 테스트 ═══

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trit_hash_deterministic() {
        let h1 = trit_hash("hello");
        let h2 = trit_hash("hello");
        assert_eq!(h1, h2);
        assert!(h1.starts_with("0t"));
        assert_eq!(h1.len(), 29); // "0t" + 27 trits
    }

    #[test]
    fn test_trit_hash_different() {
        let h1 = trit_hash("hello");
        let h2 = trit_hash("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_transaction_create_verify() {
        let tx = Transaction::new("alice", "bob", 100, 1, TxType::Transfer, "");
        assert!(tx.verify());
        assert_eq!(tx.trit(), 1);
    }

    #[test]
    fn test_merkle_root() {
        let hashes = vec![trit_hash("a"), trit_hash("b"), trit_hash("c")];
        let root = build_merkle_root(&hashes);
        assert!(root.starts_with("0t"));
        // 같은 입력 → 같은 루트
        let root2 = build_merkle_root(&hashes);
        assert_eq!(root, root2);
    }

    #[test]
    fn test_merkle_empty() {
        let root = build_merkle_root(&[]);
        assert!(root.starts_with("0t"));
    }

    #[test]
    fn test_genesis_block() {
        let genesis = Block::genesis();
        assert_eq!(genesis.index, 0);
        assert_eq!(genesis.tx_count, 1);
        assert!(genesis.verify());
    }

    #[test]
    fn test_pot_consensus() {
        let mut proof = PoTProof::new(1, 2);
        proof.add_vote("a", 1, "ok");
        proof.add_vote("b", 1, "ok");
        proof.add_vote("c", -1, "no");
        assert_eq!(proof.consensus_trit(), 1);
        assert!(proof.is_valid());
        assert!(!proof.unanimous());
    }

    #[test]
    fn test_pot_invalid() {
        let mut proof = PoTProof::new(1, 3);
        proof.add_vote("a", 1, "ok");
        // threshold=3 but only 1 vote
        assert!(!proof.is_valid());
    }

    #[test]
    fn test_pot_rejected() {
        let mut proof = PoTProof::new(1, 2);
        proof.add_vote("a", -1, "no");
        proof.add_vote("b", -1, "no");
        // consensus = T, is_valid requires >= 0
        assert!(!proof.is_valid());
    }

    #[test]
    fn test_chain_create() {
        let chain = CrownyChain::new();
        assert_eq!(chain.blocks.len(), 1);
        assert_eq!(chain.height(), 0);
        assert_eq!(chain.balance_of("treasury"), 153_000_000);
    }

    #[test]
    fn test_chain_add_validator() {
        let mut chain = CrownyChain::new();
        chain.balances.insert("alice".into(), 100_000);
        assert!(chain.add_validator("alice", "Alice", 50_000));
        assert_eq!(chain.balance_of("alice"), 50_000);
        assert_eq!(chain.validators.len(), 1);
    }

    #[test]
    fn test_chain_insufficient_stake() {
        let mut chain = CrownyChain::new();
        chain.balances.insert("bob".into(), 100);
        assert!(!chain.add_validator("bob", "Bob", 50_000));
    }

    #[test]
    fn test_tx_pool() {
        let mut pool = TxPool::new(100);
        let tx = Transaction::new("a", "b", 10, 1, TxType::Transfer, "");
        assert!(pool.add(tx));
        assert_eq!(pool.size(), 1);
        let batch = pool.take_batch(10);
        assert_eq!(batch.len(), 1);
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_chain_produce_block() {
        let mut chain = CrownyChain::new();
        chain.balances.insert("alice".into(), 1_000_000);
        chain.balances.insert("bob".into(), 500_000);
        chain.add_validator("alice", "Alice", 100_000);
        chain.add_validator("bob", "Bob", 80_000);
        chain.transfer("alice", "bob", 1000, 10);
        let block = chain.produce_block();
        assert!(block.is_some());
        assert_eq!(chain.blocks.len(), 2);
    }

    #[test]
    fn test_chain_verify() {
        let mut chain = CrownyChain::new();
        chain.balances.insert("alice".into(), 1_000_000);
        chain.balances.insert("bob".into(), 500_000);
        chain.add_validator("alice", "Alice", 100_000);
        chain.add_validator("bob", "Bob", 80_000);
        chain.transfer("alice", "bob", 1000, 10);
        chain.produce_block();
        let (valid, _) = chain.verify_chain();
        assert!(valid);
    }

    #[test]
    fn test_validator_trit() {
        let v = Validator::new("addr", "name", 1000);
        assert_eq!(v.trit(), 1);
    }

    #[test]
    fn test_block_ctp_header() {
        let genesis = Block::genesis();
        assert_eq!(genesis.ctp_header[0], 1); // consensus P
    }
}
