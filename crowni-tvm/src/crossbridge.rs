// ═══════════════════════════════════════════════════════════════
// Crowny Bridge — 크로스체인 브릿지
// EVM · Solana · Crowny Chain 간 자산 이동
// 락/민트 · 릴레이어 · 멀티시그 검증 · 수수료
// ═══════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64 }

fn trit_hash(data: &str) -> String {
    let mut h: u64 = 0xcb735a4e9f1d2b08;
    for (i, b) in data.bytes().enumerate() {
        h ^= (b as u64).wrapping_mul(0x100000001b3);
        h = h.wrapping_mul(0x517cc1b727220a95);
        h ^= (i as u64).wrapping_add(0x9e3779b97f4a7c15);
        h = h.rotate_left(17) ^ h.rotate_right(23);
    }
    let trits: String = (0..27).map(|i| match ((h >> (i * 2)) & 3) % 3 { 0 => 'P', 1 => 'O', _ => 'T' }).collect();
    format!("0t{}", trits)
}

// ═══════════════════════════════════════
// 체인 정의
// ═══════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Chain {
    Crowny,
    Ethereum,
    BSC,
    Polygon,
    Arbitrum,
    Solana,
}

impl Chain {
    pub fn chain_id(&self) -> u64 {
        match self { Self::Crowny => 3333, Self::Ethereum => 1, Self::BSC => 56,
            Self::Polygon => 137, Self::Arbitrum => 42161, Self::Solana => 99999 }
    }
    pub fn name(&self) -> &str {
        match self { Self::Crowny => "Crowny", Self::Ethereum => "Ethereum", Self::BSC => "BSC",
            Self::Polygon => "Polygon", Self::Arbitrum => "Arbitrum", Self::Solana => "Solana" }
    }
    pub fn is_evm(&self) -> bool { matches!(self, Self::Ethereum | Self::BSC | Self::Polygon | Self::Arbitrum) }
    pub fn block_time_ms(&self) -> u64 {
        match self { Self::Crowny => 3000, Self::Ethereum => 12000, Self::BSC => 3000,
            Self::Polygon => 2000, Self::Arbitrum => 250, Self::Solana => 400 }
    }
    pub fn confirmations(&self) -> u32 {
        match self { Self::Crowny => 3, Self::Ethereum => 12, Self::BSC => 15,
            Self::Polygon => 64, Self::Arbitrum => 1, Self::Solana => 32 }
    }
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name(), self.chain_id())
    }
}

// ═══════════════════════════════════════
// 브릿지 토큰 (래핑)
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct BridgeToken {
    pub symbol: String,
    pub native_chain: Chain,
    pub decimals: u8,
    pub contract_addresses: HashMap<Chain, String>,
    pub total_locked: HashMap<Chain, u64>,
    pub total_minted: HashMap<Chain, u64>,
}

impl BridgeToken {
    pub fn new(symbol: &str, native: Chain) -> Self {
        let mut contracts = HashMap::new();
        let chains = vec![Chain::Crowny, Chain::Ethereum, Chain::BSC, Chain::Polygon, Chain::Arbitrum, Chain::Solana];
        for chain in &chains {
            let addr = trit_hash(&format!("{}:{}", symbol, chain.chain_id()));
            contracts.insert(chain.clone(), addr);
        }
        Self {
            symbol: symbol.into(), native_chain: native, decimals: 18,
            contract_addresses: contracts,
            total_locked: HashMap::new(), total_minted: HashMap::new(),
        }
    }

    pub fn locked_on(&self, chain: &Chain) -> u64 { self.total_locked.get(chain).copied().unwrap_or(0) }
    pub fn minted_on(&self, chain: &Chain) -> u64 { self.total_minted.get(chain).copied().unwrap_or(0) }
}

// ═══════════════════════════════════════
// 브릿지 트랜잭션
// ═══════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum BridgeTxStatus {
    Pending,        // O: 대기
    Locked,         // P: 원본 체인 락 완료
    Relayed,        // O: 릴레이 전달됨
    Verified,       // P: 멀티시그 검증 완료
    Minted,         // P: 대상 체인 민트 완료
    Completed,      // P: 최종 완료
    Failed,         // T: 실패
    Refunded,       // T: 환불됨
}

impl BridgeTxStatus {
    pub fn trit(&self) -> i8 {
        match self {
            Self::Locked | Self::Verified | Self::Minted | Self::Completed => 1,
            Self::Pending | Self::Relayed => 0,
            Self::Failed | Self::Refunded => -1,
        }
    }
}

impl std::fmt::Display for BridgeTxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "⏳대기"), Self::Locked => write!(f, "🔒락"),
            Self::Relayed => write!(f, "📡릴레이"), Self::Verified => write!(f, "✓검증"),
            Self::Minted => write!(f, "🪙민트"), Self::Completed => write!(f, "✅완료"),
            Self::Failed => write!(f, "✗실패"), Self::Refunded => write!(f, "↩환불"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BridgeTx {
    pub id: String,
    pub sender: String,
    pub receiver: String,
    pub token: String,
    pub amount: u64,
    pub fee: u64,
    pub src_chain: Chain,
    pub dst_chain: Chain,
    pub status: BridgeTxStatus,
    pub src_tx_hash: String,
    pub dst_tx_hash: Option<String>,
    pub signatures: Vec<RelayerSig>,
    pub created_at: u64,
    pub completed_at: Option<u64>,
}

impl BridgeTx {
    pub fn trit(&self) -> i8 { self.status.trit() }

    pub fn elapsed_ms(&self) -> u64 {
        self.completed_at.unwrap_or(now_ms()) - self.created_at
    }
}

impl std::fmt::Display for BridgeTx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let trit = match self.trit() { 1 => "P", -1 => "T", _ => "O" };
        write!(f, "[{}] {} {} {} → {} | {} → {} | {}",
            trit, self.amount, self.token, self.src_chain.name(), self.dst_chain.name(),
            self.sender, self.receiver, self.status)
    }
}

// ═══════════════════════════════════════
// 릴레이어 (멀티시그 검증)
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Relayer {
    pub name: String,
    pub address: String,
    pub stake: u64,
    pub reputation: f64,
    pub active: bool,
    pub txs_relayed: u64,
    pub chains_supported: Vec<Chain>,
}

impl Relayer {
    pub fn new(name: &str, stake: u64, chains: Vec<Chain>) -> Self {
        Self {
            name: name.into(),
            address: trit_hash(&format!("relayer:{}", name)),
            stake, reputation: 1.0, active: true, txs_relayed: 0,
            chains_supported: chains,
        }
    }

    pub fn trit(&self) -> i8 {
        if self.reputation > 0.7 && self.active { 1 }
        else if self.reputation > 0.3 { 0 }
        else { -1 }
    }

    pub fn supports(&self, chain: &Chain) -> bool { self.chains_supported.contains(chain) }
}

impl std::fmt::Display for Relayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let trit = match self.trit() { 1 => "P", -1 => "T", _ => "O" };
        let status = if self.active { "●" } else { "○" };
        let chains: Vec<&str> = self.chains_supported.iter().map(|c| c.name()).collect();
        write!(f, "[{}]{} {} — {}K staked | rep:{:.2} | {} relayed | [{}]",
            trit, status, self.name, self.stake / 1000, self.reputation,
            self.txs_relayed, chains.join(","))
    }
}

#[derive(Debug, Clone)]
pub struct RelayerSig {
    pub relayer: String,
    pub approved: bool,
    pub signature: String,
    pub timestamp: u64,
}

// ═══════════════════════════════════════
// 브릿지 본체
// ═══════════════════════════════════════

pub struct CrownyBridge {
    pub tokens: HashMap<String, BridgeToken>,
    pub relayers: Vec<Relayer>,
    pub transactions: Vec<BridgeTx>,
    pub tx_counter: u64,
    pub multisig_threshold: usize,  // 필요 서명 수
    pub fee_bps: u64,               // 수수료 (basis points)
    pub total_volume: u64,
    pub total_fees: u64,
    pub balances: HashMap<String, HashMap<String, u64>>,  // user → token → balance
}

impl CrownyBridge {
    pub fn new() -> Self {
        let mut b = Self {
            tokens: HashMap::new(), relayers: Vec::new(),
            transactions: Vec::new(), tx_counter: 0,
            multisig_threshold: 2, fee_bps: 10, // 0.1%
            total_volume: 0, total_fees: 0,
            balances: HashMap::new(),
        };
        // 기본 토큰
        b.register_token("CRWN", Chain::Crowny);
        b.register_token("ETH", Chain::Ethereum);
        b.register_token("BNB", Chain::BSC);
        b.register_token("MATIC", Chain::Polygon);
        b.register_token("SOL", Chain::Solana);
        b.register_token("USDT", Chain::Ethereum);
        b
    }

    pub fn register_token(&mut self, symbol: &str, native: Chain) {
        self.tokens.insert(symbol.into(), BridgeToken::new(symbol, native));
    }

    pub fn add_relayer(&mut self, name: &str, stake: u64, chains: Vec<Chain>) {
        self.relayers.push(Relayer::new(name, stake, chains));
    }

    pub fn mint(&mut self, user: &str, token: &str, amount: u64) {
        *self.balances.entry(user.into()).or_default().entry(token.into()).or_insert(0) += amount;
    }

    pub fn balance(&self, user: &str, token: &str) -> u64 {
        self.balances.get(user).and_then(|m| m.get(token)).copied().unwrap_or(0)
    }

    /// 브릿지 전송 시작
    pub fn initiate_transfer(
        &mut self, sender: &str, receiver: &str, token: &str,
        amount: u64, src: Chain, dst: Chain,
    ) -> Result<usize, String> {
        // 검증
        if src == dst { return Err("동일 체인".into()); }
        if !self.tokens.contains_key(token) { return Err(format!("미지원 토큰: {}", token)); }
        let bal = self.balance(sender, token);
        if bal < amount { return Err(format!("잔액 부족: {} {} (보유: {})", token, amount, bal)); }

        let fee = amount * self.fee_bps / 10000;
        let net_amount = amount - fee;

        // 잔액 차감 (락)
        *self.balances.get_mut(sender).unwrap().get_mut(token).unwrap() -= amount;

        // 토큰 락 기록
        if let Some(bt) = self.tokens.get_mut(token) {
            *bt.total_locked.entry(src.clone()).or_insert(0) += net_amount;
        }

        let tx_id = format!("BRIDGE-{:06}", self.tx_counter);
        self.tx_counter += 1;

        let src_hash = trit_hash(&format!("lock:{}:{}:{}:{}", sender, token, amount, now_ms()));

        self.transactions.push(BridgeTx {
            id: tx_id, sender: sender.into(), receiver: receiver.into(),
            token: token.into(), amount: net_amount, fee,
            src_chain: src, dst_chain: dst, status: BridgeTxStatus::Locked,
            src_tx_hash: src_hash, dst_tx_hash: None,
            signatures: Vec::new(), created_at: now_ms(), completed_at: None,
        });

        self.total_volume += amount;
        self.total_fees += fee;

        Ok(self.transactions.len() - 1)
    }

    /// 릴레이어가 트랜잭션 검증/서명
    pub fn relay_verify(&mut self, tx_idx: usize, relayer_idx: usize, approved: bool) -> Result<(), String> {
        let relayer = self.relayers.get(relayer_idx).ok_or("릴레이어 없음")?.clone();
        let tx = self.transactions.get_mut(tx_idx).ok_or("TX 없음")?;

        if !relayer.supports(&tx.src_chain) && !relayer.supports(&tx.dst_chain) {
            return Err(format!("{} 미지원 체인", relayer.name));
        }

        let sig = RelayerSig {
            relayer: relayer.name.clone(),
            approved,
            signature: trit_hash(&format!("sig:{}:{}:{}", relayer.name, tx.id, approved)),
            timestamp: now_ms(),
        };
        tx.signatures.push(sig);

        if tx.status == BridgeTxStatus::Locked { tx.status = BridgeTxStatus::Relayed; }

        // 멀티시그 확인
        let approvals = tx.signatures.iter().filter(|s| s.approved).count();
        if approvals >= self.multisig_threshold {
            tx.status = BridgeTxStatus::Verified;
        }

        // 릴레이어 통계
        if let Some(r) = self.relayers.get_mut(relayer_idx) {
            r.txs_relayed += 1;
        }

        Ok(())
    }

    /// 대상 체인에 민트 실행
    pub fn execute_mint(&mut self, tx_idx: usize) -> Result<(), String> {
        let tx = self.transactions.get(tx_idx).ok_or("TX 없음")?;
        if tx.status != BridgeTxStatus::Verified { return Err("미검증 TX".into()); }

        let token = tx.token.clone();
        let receiver = tx.receiver.clone();
        let amount = tx.amount;
        let dst = tx.dst_chain.clone();

        // 민트
        *self.balances.entry(receiver).or_default().entry(token.clone()).or_insert(0) += amount;

        if let Some(bt) = self.tokens.get_mut(&token) {
            *bt.total_minted.entry(dst).or_insert(0) += amount;
        }

        let tx = self.transactions.get_mut(tx_idx).unwrap();
        tx.dst_tx_hash = Some(trit_hash(&format!("mint:{}:{}:{}", tx.id, amount, now_ms())));
        tx.status = BridgeTxStatus::Completed;
        tx.completed_at = Some(now_ms());

        Ok(())
    }

    /// 전체 프로세스 (락 → 릴레이 → 검증 → 민트)
    pub fn bridge_transfer(
        &mut self, sender: &str, receiver: &str, token: &str,
        amount: u64, src: Chain, dst: Chain,
    ) -> Result<BridgeTx, String> {
        let tx_idx = self.initiate_transfer(sender, receiver, token, amount, src, dst)?;

        // 릴레이어 자동 서명
        let relayer_count = self.relayers.len();
        for ri in 0..relayer_count {
            let approved = self.relayers[ri].reputation > 0.3;
            self.relay_verify(tx_idx, ri, approved).ok();
        }

        // 민트
        if self.transactions[tx_idx].status == BridgeTxStatus::Verified {
            self.execute_mint(tx_idx)?;
        }

        Ok(self.transactions[tx_idx].clone())
    }

    pub fn supported_routes(&self) -> Vec<(Chain, Chain)> {
        let chains = vec![Chain::Crowny, Chain::Ethereum, Chain::BSC, Chain::Polygon, Chain::Arbitrum, Chain::Solana];
        let mut routes = Vec::new();
        for src in &chains {
            for dst in &chains {
                if src != dst { routes.push((src.clone(), dst.clone())); }
            }
        }
        routes
    }

    pub fn summary(&self) -> String {
        let completed = self.transactions.iter().filter(|t| t.status == BridgeTxStatus::Completed).count();
        let pending = self.transactions.iter().filter(|t| t.status != BridgeTxStatus::Completed && t.status != BridgeTxStatus::Failed).count();
        format!(
            "CrownyBridge\n  토큰: {} | 릴레이어: {} | TX: {} (완료:{}, 대기:{})\n  볼륨: {} | 수수료: {} | 라우트: {}",
            self.tokens.len(), self.relayers.len(), self.transactions.len(),
            completed, pending, self.total_volume, self.total_fees, self.supported_routes().len()
        )
    }
}

// ═══ 데모 ═══

pub fn demo_bridge() {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║  Crowny Bridge — 크로스체인 브릿지              ║");
    println!("║  EVM · Solana · Crowny 간 자산 이동             ║");
    println!("╚═══════════════════════════════════════════════╝");
    println!();

    let mut bridge = CrownyBridge::new();

    // 1. 지원 체인
    println!("━━━ 1. 지원 체인 ━━━");
    let chains = vec![Chain::Crowny, Chain::Ethereum, Chain::BSC, Chain::Polygon, Chain::Arbitrum, Chain::Solana];
    for c in &chains {
        let evm = if c.is_evm() { "EVM" } else { "Non-EVM" };
        println!("  {} — {} | 블록: {}ms | 확인: {} blocks",
            c, evm, c.block_time_ms(), c.confirmations());
    }
    println!("  총 라우트: {} 경로", bridge.supported_routes().len());
    println!();

    // 2. 릴레이어 등록
    println!("━━━ 2. 릴레이어 네트워크 ━━━");
    bridge.add_relayer("Alpha-Relay", 500_000,
        vec![Chain::Crowny, Chain::Ethereum, Chain::BSC, Chain::Polygon, Chain::Arbitrum, Chain::Solana]);
    bridge.add_relayer("Beta-Relay", 300_000,
        vec![Chain::Crowny, Chain::Ethereum, Chain::BSC, Chain::Polygon]);
    bridge.add_relayer("Gamma-Relay", 200_000,
        vec![Chain::Crowny, Chain::Ethereum, Chain::Solana]);
    for r in &bridge.relayers { println!("  {}", r); }
    println!("  멀티시그 임계값: {}/{}", bridge.multisig_threshold, bridge.relayers.len());
    println!();

    // 3. 잔액 배정
    println!("━━━ 3. 초기 잔액 ━━━");
    bridge.mint("alice", "CRWN", 1_000_000);
    bridge.mint("alice", "ETH", 100);
    bridge.mint("bob", "CRWN", 500_000);
    bridge.mint("bob", "SOL", 1_000);
    bridge.mint("carol", "ETH", 50);
    bridge.mint("carol", "BNB", 200);
    let users = vec!["alice", "bob", "carol"];
    for u in &users {
        let bals = bridge.balances.get(*u).unwrap();
        let parts: Vec<String> = bals.iter().filter(|(_, v)| **v > 0).map(|(t, v)| format!("{} {}", v, t)).collect();
        println!("  {} — {}", u, parts.join(", "));
    }
    println!();

    // 4. 브릿지 전송
    println!("━━━ 4. 크로스체인 전송 ━━━");
    let transfers = vec![
        ("alice", "alice", "CRWN", 100_000, Chain::Crowny, Chain::Ethereum, "CRWN → Ethereum"),
        ("alice", "bob", "ETH", 10, Chain::Ethereum, Chain::Crowny, "ETH → Crowny"),
        ("bob", "carol", "CRWN", 50_000, Chain::Crowny, Chain::Solana, "CRWN → Solana"),
        ("bob", "bob", "SOL", 200, Chain::Solana, Chain::Crowny, "SOL → Crowny"),
        ("carol", "alice", "BNB", 50, Chain::BSC, Chain::Crowny, "BNB → Crowny"),
        ("alice", "carol", "CRWN", 200_000, Chain::Crowny, Chain::Polygon, "CRWN → Polygon"),
        ("carol", "bob", "ETH", 20, Chain::Ethereum, Chain::Arbitrum, "ETH → Arbitrum"),
    ];

    for (sender, receiver, token, amount, src, dst, desc) in &transfers {
        println!("  ▸ {} ({})", desc, sender);
        match bridge.bridge_transfer(sender, receiver, token, *amount, src.clone(), dst.clone()) {
            Ok(tx) => {
                println!("    {}", tx);
                println!("    서명: {} | 수수료: {} {} | {}ms",
                    tx.signatures.len(), tx.fee, token, tx.elapsed_ms());
                if let Some(ref dst_hash) = tx.dst_tx_hash {
                    let h: String = dst_hash.chars().take(20).collect();
                    println!("    src: {}... → dst: {}...",
                        &tx.src_tx_hash.chars().take(20).collect::<String>(), h);
                }
            }
            Err(e) => println!("    [T] 실패: {}", e),
        }
        println!();
    }

    // 5. 토큰 락/민트 현황
    println!("━━━ 5. 토큰 락/민트 현황 ━━━");
    for (symbol, bt) in &bridge.tokens {
        let locked: u64 = bt.total_locked.values().sum();
        let minted: u64 = bt.total_minted.values().sum();
        if locked > 0 || minted > 0 {
            println!("  {} — 원본: {} | 총 락: {} | 총 민트: {}", symbol, bt.native_chain.name(), locked, minted);
            for (chain, amt) in &bt.total_locked {
                if *amt > 0 { println!("    🔒 {} 에서 {} 락", chain.name(), amt); }
            }
            for (chain, amt) in &bt.total_minted {
                if *amt > 0 { println!("    🪙 {} 에서 {} 민트", chain.name(), amt); }
            }
        }
    }
    println!();

    // 6. 릴레이어 통계
    println!("━━━ 6. 릴레이어 통계 ━━━");
    for r in &bridge.relayers {
        println!("  {}", r);
    }
    println!();

    // 7. 최종 잔액
    println!("━━━ 7. 최종 잔액 ━━━");
    for u in &users {
        if let Some(bals) = bridge.balances.get(*u) {
            let parts: Vec<String> = bals.iter().filter(|(_, v)| **v > 0).map(|(t, v)| format!("{} {}", v, t)).collect();
            println!("  {} — {}", u, parts.join(", "));
        }
    }
    println!();

    // 8. 요약
    println!("━━━ 8. 브릿지 요약 ━━━");
    println!("{}", bridge.summary());
    println!();
    println!("✓ Crowny Bridge 데모 완료");
}

// ═══ 테스트 ═══

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_properties() {
        assert_eq!(Chain::Crowny.chain_id(), 3333);
        assert!(Chain::Ethereum.is_evm());
        assert!(!Chain::Solana.is_evm());
        assert!(!Chain::Crowny.is_evm());
    }

    #[test]
    fn test_bridge_token_create() {
        let bt = BridgeToken::new("CRWN", Chain::Crowny);
        assert_eq!(bt.symbol, "CRWN");
        assert!(bt.contract_addresses.len() >= 6);
    }

    #[test]
    fn test_relayer_create() {
        let r = Relayer::new("Test", 100_000, vec![Chain::Crowny, Chain::Ethereum]);
        assert_eq!(r.trit(), 1);
        assert!(r.supports(&Chain::Crowny));
        assert!(!r.supports(&Chain::Solana));
    }

    #[test]
    fn test_bridge_initiate() {
        let mut bridge = CrownyBridge::new();
        bridge.mint("alice", "CRWN", 100_000);
        bridge.add_relayer("R1", 100_000, vec![Chain::Crowny, Chain::Ethereum]);
        let idx = bridge.initiate_transfer("alice", "bob", "CRWN", 10_000, Chain::Crowny, Chain::Ethereum);
        assert!(idx.is_ok());
        assert_eq!(bridge.transactions.len(), 1);
        assert_eq!(bridge.transactions[0].status, BridgeTxStatus::Locked);
    }

    #[test]
    fn test_bridge_same_chain_error() {
        let mut bridge = CrownyBridge::new();
        bridge.mint("alice", "CRWN", 100_000);
        let r = bridge.initiate_transfer("alice", "bob", "CRWN", 1000, Chain::Crowny, Chain::Crowny);
        assert!(r.is_err());
    }

    #[test]
    fn test_bridge_insufficient_balance() {
        let mut bridge = CrownyBridge::new();
        bridge.mint("alice", "CRWN", 100);
        let r = bridge.initiate_transfer("alice", "bob", "CRWN", 10_000, Chain::Crowny, Chain::Ethereum);
        assert!(r.is_err());
    }

    #[test]
    fn test_bridge_full_flow() {
        let mut bridge = CrownyBridge::new();
        bridge.mint("alice", "CRWN", 100_000);
        bridge.add_relayer("R1", 100_000, vec![Chain::Crowny, Chain::Ethereum]);
        bridge.add_relayer("R2", 80_000, vec![Chain::Crowny, Chain::Ethereum]);
        let result = bridge.bridge_transfer("alice", "bob", "CRWN", 10_000, Chain::Crowny, Chain::Ethereum);
        assert!(result.is_ok());
        let tx = result.unwrap();
        assert_eq!(tx.status, BridgeTxStatus::Completed);
        assert!(bridge.balance("bob", "CRWN") > 0);
    }

    #[test]
    fn test_bridge_fee() {
        let mut bridge = CrownyBridge::new();
        bridge.mint("alice", "CRWN", 100_000);
        bridge.add_relayer("R1", 100_000, vec![Chain::Crowny, Chain::Ethereum]);
        bridge.add_relayer("R2", 80_000, vec![Chain::Crowny, Chain::Ethereum]);
        bridge.bridge_transfer("alice", "bob", "CRWN", 10_000, Chain::Crowny, Chain::Ethereum).unwrap();
        // 0.1% fee = 10
        assert_eq!(bridge.total_fees, 10);
        assert_eq!(bridge.balance("bob", "CRWN"), 9990);
    }

    #[test]
    fn test_multisig_threshold() {
        let mut bridge = CrownyBridge::new();
        bridge.mint("alice", "CRWN", 100_000);
        bridge.add_relayer("R1", 100_000, vec![Chain::Crowny, Chain::Ethereum]);
        // Only 1 relayer, threshold is 2 → should NOT complete
        let tx_idx = bridge.initiate_transfer("alice", "bob", "CRWN", 1000, Chain::Crowny, Chain::Ethereum).unwrap();
        bridge.relay_verify(tx_idx, 0, true).unwrap();
        assert_ne!(bridge.transactions[tx_idx].status, BridgeTxStatus::Verified);
    }

    #[test]
    fn test_supported_routes() {
        let bridge = CrownyBridge::new();
        let routes = bridge.supported_routes();
        assert_eq!(routes.len(), 30); // 6 chains, 6*5 = 30
    }

    #[test]
    fn test_bridge_tx_status_trit() {
        assert_eq!(BridgeTxStatus::Completed.trit(), 1);
        assert_eq!(BridgeTxStatus::Pending.trit(), 0);
        assert_eq!(BridgeTxStatus::Failed.trit(), -1);
    }

    #[test]
    fn test_bridge_summary() {
        let bridge = CrownyBridge::new();
        let s = bridge.summary();
        assert!(s.contains("CrownyBridge"));
        assert!(s.contains("토큰: 6"));
    }
}
