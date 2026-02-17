///! ┌─────────────────────────────────────────┐
///! │  Crowny Token System (3진 토큰)           │
///! │  균형3진법 기반 토큰 발행/전송/스테이킹     │
///! └─────────────────────────────────────────┘

use std::collections::HashMap;
use std::fmt;

// ═══════════════════════════════════════════════
// 토큰 타입
// ═══════════════════════════════════════════════

/// 3진 토큰 — 모든 값은 균형3진수로 표현 가능
#[derive(Debug, Clone)]
pub struct CrownyToken {
    pub name: String,
    pub symbol: String,
    pub total_supply: u64,
    pub decimals: u8,
    pub issuer: String,
    pub created_at: u64,
    pub trit_policy: TritPolicy,
}

/// 3진 정책 — 토큰 거버넌스
#[derive(Debug, Clone)]
pub struct TritPolicy {
    pub mintable: TritPerm,      // 추가 발행 가능?
    pub burnable: TritPerm,      // 소각 가능?
    pub transferable: TritPerm,  // 전송 가능?
    pub stakeable: TritPerm,     // 스테이킹 가능?
    pub consensus_required: bool, // 합의 필요?
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TritPerm {
    Allow,   // P: 허용
    Pending, // O: 거버넌스 투표 필요
    Deny,    // T: 거부
}

impl fmt::Display for TritPerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TritPerm::Allow => write!(f, "P(허용)"),
            TritPerm::Pending => write!(f, "O(투표)"),
            TritPerm::Deny => write!(f, "T(거부)"),
        }
    }
}

impl Default for TritPolicy {
    fn default() -> Self {
        Self {
            mintable: TritPerm::Deny,
            burnable: TritPerm::Allow,
            transferable: TritPerm::Allow,
            stakeable: TritPerm::Allow,
            consensus_required: false,
        }
    }
}

// ═══════════════════════════════════════════════
// 지갑
// ═══════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Wallet {
    pub address: String,
    pub balance: u64,
    pub staked: u64,
    pub nonce: u64,
    pub created_at: u64,
}

impl Wallet {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
            balance: 0,
            staked: 0,
            nonce: 0,
            created_at: now_ms(),
        }
    }

    pub fn available(&self) -> u64 {
        self.balance.saturating_sub(self.staked)
    }
}

// ═══════════════════════════════════════════════
// 전송 트랜잭션
// ═══════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct TokenTx {
    pub id: u64,
    pub tx_type: TokenTxType,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: u64,
    pub state: TxState,
    pub trit_header: [i8; 9], // CTP 9-Trit
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenTxType {
    Transfer,
    Mint,
    Burn,
    Stake,
    Unstake,
    ContractCall,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TxState {
    Pending,   // O
    Confirmed, // P
    Rejected,  // T
}

impl fmt::Display for TxState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TxState::Pending => write!(f, "O(보류)"),
            TxState::Confirmed => write!(f, "P(확인)"),
            TxState::Rejected => write!(f, "T(거부)"),
        }
    }
}

// ═══════════════════════════════════════════════
// 스마트 컨트랙트 (3진)
// ═══════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct TritContract {
    pub id: u64,
    pub name: String,
    pub owner: String,
    pub code: String,       // 한선어 소스
    pub state: ContractState,
    pub storage: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContractState {
    Active,     // P
    Paused,     // O
    Terminated, // T
}

impl fmt::Display for ContractState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractState::Active => write!(f, "P(활성)"),
            ContractState::Paused => write!(f, "O(일시정지)"),
            ContractState::Terminated => write!(f, "T(종료)"),
        }
    }
}

// ═══════════════════════════════════════════════
// 토큰 엔진
// ═══════════════════════════════════════════════

pub struct TokenEngine {
    pub token: CrownyToken,
    pub wallets: HashMap<String, Wallet>,
    pub transactions: Vec<TokenTx>,
    pub contracts: Vec<TritContract>,
    tx_counter: u64,
    contract_counter: u64,
}

impl TokenEngine {
    pub fn new(name: &str, symbol: &str, supply: u64, issuer: &str) -> Self {
        let token = CrownyToken {
            name: name.to_string(),
            symbol: symbol.to_string(),
            total_supply: supply,
            decimals: 9,
            issuer: issuer.to_string(),
            created_at: now_ms(),
            trit_policy: TritPolicy::default(),
        };

        let mut wallets = HashMap::new();
        let mut issuer_wallet = Wallet::new(issuer);
        issuer_wallet.balance = supply;
        wallets.insert(issuer.to_string(), issuer_wallet);

        Self {
            token,
            wallets,
            transactions: Vec::new(),
            contracts: Vec::new(),
            tx_counter: 0,
            contract_counter: 0,
        }
    }

    /// 전송
    pub fn transfer(&mut self, from: &str, to: &str, amount: u64) -> TokenTx {
        self.tx_counter += 1;
        let fee = amount / 1000; // 0.1% 수수료

        // 정책 검사
        if self.token.trit_policy.transferable == TritPerm::Deny {
            return self.create_tx(TokenTxType::Transfer, from, to, amount, fee, TxState::Rejected);
        }

        // 잔고 검사
        let sender = self.wallets.get(from);
        if sender.is_none() || sender.unwrap().available() < amount + fee {
            return self.create_tx(TokenTxType::Transfer, from, to, amount, fee, TxState::Rejected);
        }

        // 실행
        if let Some(s) = self.wallets.get_mut(from) {
            s.balance = s.balance.saturating_sub(amount + fee);
            s.nonce += 1;
        }

        self.wallets.entry(to.to_string())
            .or_insert_with(|| Wallet::new(to))
            .balance += amount;

        self.create_tx(TokenTxType::Transfer, from, to, amount, fee, TxState::Confirmed)
    }

    /// 발행 (민트)
    pub fn mint(&mut self, to: &str, amount: u64) -> TokenTx {
        self.tx_counter += 1;

        if self.token.trit_policy.mintable == TritPerm::Deny {
            return self.create_tx(TokenTxType::Mint, "system", to, amount, 0, TxState::Rejected);
        }

        self.token.total_supply += amount;
        self.wallets.entry(to.to_string())
            .or_insert_with(|| Wallet::new(to))
            .balance += amount;

        self.create_tx(TokenTxType::Mint, "system", to, amount, 0, TxState::Confirmed)
    }

    /// 소각 (번)
    pub fn burn(&mut self, from: &str, amount: u64) -> TokenTx {
        self.tx_counter += 1;

        if self.token.trit_policy.burnable == TritPerm::Deny {
            return self.create_tx(TokenTxType::Burn, from, "null", amount, 0, TxState::Rejected);
        }

        let sender = self.wallets.get(from);
        if sender.is_none() || sender.unwrap().available() < amount {
            return self.create_tx(TokenTxType::Burn, from, "null", amount, 0, TxState::Rejected);
        }

        if let Some(s) = self.wallets.get_mut(from) {
            s.balance = s.balance.saturating_sub(amount);
        }
        self.token.total_supply = self.token.total_supply.saturating_sub(amount);

        self.create_tx(TokenTxType::Burn, from, "null", amount, 0, TxState::Confirmed)
    }

    /// 스테이킹
    pub fn stake(&mut self, who: &str, amount: u64) -> TokenTx {
        self.tx_counter += 1;

        if self.token.trit_policy.stakeable == TritPerm::Deny {
            return self.create_tx(TokenTxType::Stake, who, "staking", amount, 0, TxState::Rejected);
        }

        let wallet = self.wallets.get(who);
        if wallet.is_none() || wallet.unwrap().available() < amount {
            return self.create_tx(TokenTxType::Stake, who, "staking", amount, 0, TxState::Rejected);
        }

        if let Some(w) = self.wallets.get_mut(who) {
            w.staked += amount;
        }

        self.create_tx(TokenTxType::Stake, who, "staking", amount, 0, TxState::Confirmed)
    }

    /// 언스테이킹
    pub fn unstake(&mut self, who: &str, amount: u64) -> TokenTx {
        self.tx_counter += 1;

        let wallet = self.wallets.get(who);
        if wallet.is_none() || wallet.unwrap().staked < amount {
            return self.create_tx(TokenTxType::Unstake, "staking", who, amount, 0, TxState::Rejected);
        }

        if let Some(w) = self.wallets.get_mut(who) {
            w.staked = w.staked.saturating_sub(amount);
        }

        self.create_tx(TokenTxType::Unstake, "staking", who, amount, 0, TxState::Confirmed)
    }

    /// 스마트 컨트랙트 배포
    pub fn deploy_contract(&mut self, name: &str, owner: &str, code: &str) -> &TritContract {
        self.contract_counter += 1;
        let contract = TritContract {
            id: self.contract_counter,
            name: name.to_string(),
            owner: owner.to_string(),
            code: code.to_string(),
            state: ContractState::Active,
            storage: HashMap::new(),
        };
        self.contracts.push(contract);
        self.contracts.last().unwrap()
    }

    /// 잔고 조회
    pub fn balance_of(&self, address: &str) -> u64 {
        self.wallets.get(address).map(|w| w.balance).unwrap_or(0)
    }

    /// 스테이킹 조회
    pub fn staked_of(&self, address: &str) -> u64 {
        self.wallets.get(address).map(|w| w.staked).unwrap_or(0)
    }

    /// 통계
    pub fn stats(&self) -> TokenStats {
        let total_staked: u64 = self.wallets.values().map(|w| w.staked).sum();
        let confirmed = self.transactions.iter().filter(|t| t.state == TxState::Confirmed).count();
        let rejected = self.transactions.iter().filter(|t| t.state == TxState::Rejected).count();

        TokenStats {
            total_supply: self.token.total_supply,
            holders: self.wallets.len(),
            total_staked,
            total_transactions: self.transactions.len(),
            confirmed_tx: confirmed,
            rejected_tx: rejected,
            contracts: self.contracts.len(),
        }
    }

    fn create_tx(&mut self, tx_type: TokenTxType, from: &str, to: &str, amount: u64, fee: u64, state: TxState) -> TokenTx {
        let trit_header = match &state {
            TxState::Confirmed => [1, 1, 1, 0, 0, 0, 0, 0, 0],
            TxState::Pending => [0, 0, 0, 0, 0, 0, 0, 0, 0],
            TxState::Rejected => [-1, -1, -1, 0, 0, 0, 0, 0, 0],
        };

        let tx = TokenTx {
            id: self.tx_counter,
            tx_type,
            from: from.to_string(),
            to: to.to_string(),
            amount,
            fee,
            timestamp: now_ms(),
            state,
            trit_header,
        };

        self.transactions.push(tx.clone());
        tx
    }
}

#[derive(Debug)]
pub struct TokenStats {
    pub total_supply: u64,
    pub holders: usize,
    pub total_staked: u64,
    pub total_transactions: usize,
    pub confirmed_tx: usize,
    pub rejected_tx: usize,
    pub contracts: usize,
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

// ═══════════════════════════════════════════════
// 데모
// ═══════════════════════════════════════════════

pub fn demo_token() {
    println!("╔═══════════════════════════════════════════╗");
    println!("║   CROWNY TOKEN SYSTEM (3진 토큰)           ║");
    println!("╚═══════════════════════════════════════════╝");
    println!();

    // 토큰 생성
    let mut engine = TokenEngine::new(
        "Crowny Coin",
        "CRWN",
        1_000_000_000, // 10억
        "genesis",
    );

    println!("◆ 토큰: {} ({})", engine.token.name, engine.token.symbol);
    println!("  총 공급: {}", engine.token.total_supply);
    println!("  정책:");
    println!("    발행: {}", engine.token.trit_policy.mintable);
    println!("    소각: {}", engine.token.trit_policy.burnable);
    println!("    전송: {}", engine.token.trit_policy.transferable);
    println!("    스테이킹: {}", engine.token.trit_policy.stakeable);
    println!();

    // 전송
    println!("── 전송 ──");
    let tx1 = engine.transfer("genesis", "alice", 100_000);
    println!("  TX#{}: genesis → alice 100,000 CRWN [{}]", tx1.id, tx1.state);

    let tx2 = engine.transfer("genesis", "bob", 50_000);
    println!("  TX#{}: genesis → bob 50,000 CRWN [{}]", tx2.id, tx2.state);

    let tx3 = engine.transfer("alice", "carol", 30_000);
    println!("  TX#{}: alice → carol 30,000 CRWN [{}]", tx3.id, tx3.state);
    println!();

    // 잔고
    println!("── 잔고 ──");
    for addr in &["genesis", "alice", "bob", "carol"] {
        println!("  {}: {} CRWN", addr, engine.balance_of(addr));
    }
    println!();

    // 스테이킹
    println!("── 스테이킹 ──");
    let s1 = engine.stake("alice", 20_000);
    println!("  alice 스테이킹 20,000 CRWN [{}]", s1.state);
    let s2 = engine.stake("bob", 10_000);
    println!("  bob 스테이킹 10,000 CRWN [{}]", s2.state);
    println!("  alice 사용가능: {} CRWN (스테이킹: {})", 
        engine.wallets.get("alice").unwrap().available(),
        engine.staked_of("alice"));
    println!();

    // 소각
    println!("── 소각 ──");
    let b1 = engine.burn("bob", 5_000);
    println!("  bob 소각 5,000 CRWN [{}]", b1.state);
    println!("  총 공급량: {}", engine.token.total_supply);
    println!();

    // 스마트 컨트랙트
    println!("── 스마트 컨트랙트 ──");
    let contract = engine.deploy_contract(
        "AI 투표 컨트랙트",
        "alice",
        "질문해 \"투표 결과?\"\n보여줘\n끝",
    );
    println!("  #{} \"{}\" by {} [{}]", contract.id, contract.name, contract.owner, contract.state);
    println!("  코드: {}...", contract.code.chars().take(10).collect::<String>());
    println!();

    // 실패 테스트
    println!("── 실패 테스트 ──");
    let fail = engine.transfer("nobody", "alice", 1_000);
    println!("  잔고 부족: [{}]", fail.state);

    // CTP 헤더
    let last = engine.transactions.last().unwrap();
    let header: String = last.trit_header.iter().map(|t| match t {
        1 => 'P', -1 => 'T', _ => 'O',
    }).collect();
    println!("  CTP 헤더: {}", header);
    println!();

    // 통계
    let stats = engine.stats();
    println!("── 통계 ──");
    println!("  총 공급: {}", stats.total_supply);
    println!("  홀더 수: {}", stats.holders);
    println!("  스테이킹: {}", stats.total_staked);
    println!("  트랜잭션: {} (확인: {}, 거부: {})",
        stats.total_transactions, stats.confirmed_tx, stats.rejected_tx);
    println!("  컨트랙트: {}", stats.contracts);
    println!();
    println!("✓ 토큰 시스템 데모 완료");
}

// ═══════════════════════════════════════════════
// 테스트
// ═══════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_creation() {
        let engine = TokenEngine::new("Test", "TST", 1000, "admin");
        assert_eq!(engine.token.symbol, "TST");
        assert_eq!(engine.token.total_supply, 1000);
        assert_eq!(engine.balance_of("admin"), 1000);
    }

    #[test]
    fn test_transfer() {
        let mut engine = TokenEngine::new("Test", "TST", 1000, "admin");
        let tx = engine.transfer("admin", "user1", 200);
        assert_eq!(tx.state, TxState::Confirmed);
        assert_eq!(engine.balance_of("user1"), 200);
    }

    #[test]
    fn test_transfer_insufficient() {
        let mut engine = TokenEngine::new("Test", "TST", 100, "admin");
        let tx = engine.transfer("admin", "user1", 200);
        assert_eq!(tx.state, TxState::Rejected);
    }

    #[test]
    fn test_transfer_nonexistent() {
        let mut engine = TokenEngine::new("Test", "TST", 1000, "admin");
        let tx = engine.transfer("nobody", "user1", 100);
        assert_eq!(tx.state, TxState::Rejected);
    }

    #[test]
    fn test_stake_unstake() {
        let mut engine = TokenEngine::new("Test", "TST", 1000, "admin");
        engine.transfer("admin", "user1", 500);

        let s = engine.stake("user1", 200);
        assert_eq!(s.state, TxState::Confirmed);
        assert_eq!(engine.staked_of("user1"), 200);

        // can't transfer staked amount
        let w = engine.wallets.get("user1").unwrap();
        assert_eq!(w.available(), 300);

        let u = engine.unstake("user1", 100);
        assert_eq!(u.state, TxState::Confirmed);
        assert_eq!(engine.staked_of("user1"), 100);
    }

    #[test]
    fn test_burn() {
        let mut engine = TokenEngine::new("Test", "TST", 1000, "admin");
        let b = engine.burn("admin", 100);
        assert_eq!(b.state, TxState::Confirmed);
        assert_eq!(engine.token.total_supply, 900);
        assert_eq!(engine.balance_of("admin"), 900);
    }

    #[test]
    fn test_contract_deploy() {
        let mut engine = TokenEngine::new("Test", "TST", 1000, "admin");
        engine.deploy_contract("TestContract", "admin", "끝");
        assert_eq!(engine.contracts.len(), 1);
        assert_eq!(engine.contracts[0].state, ContractState::Active);
    }

    #[test]
    fn test_stats() {
        let mut engine = TokenEngine::new("Test", "TST", 1000, "admin");
        engine.transfer("admin", "user1", 100);
        engine.transfer("admin", "user2", 200);
        let stats = engine.stats();
        assert_eq!(stats.holders, 3);
        assert_eq!(stats.confirmed_tx, 2);
    }

    #[test]
    fn test_trit_policy_deny() {
        let mut engine = TokenEngine::new("Test", "TST", 1000, "admin");
        engine.token.trit_policy.transferable = TritPerm::Deny;
        let tx = engine.transfer("admin", "user1", 100);
        assert_eq!(tx.state, TxState::Rejected);
    }

    #[test]
    fn test_fee_deduction() {
        let mut engine = TokenEngine::new("Test", "TST", 100_000, "admin");
        engine.transfer("admin", "user1", 10_000);
        // fee = 10_000 / 1000 = 10
        // admin should have 100_000 - 10_000 - 10 = 89,990
        assert_eq!(engine.balance_of("admin"), 89990);
    }
}
