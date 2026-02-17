///! ═══════════════════════════════════════════════════
///! Trit Transaction Engine — 균형3진 트랜잭션 엔진
///! ═══════════════════════════════════════════════════
///!
///! 모든 상태 변경은 3진 트랜잭션:
///!   P(+1) = Commit  (확정)
///!   O( 0) = Pending (보류/진행중)
///!   T(-1) = Rollback (취소/복원)
///!
///! WAL(Write-Ahead Log) + 3진 상태 머신
///! 2진 DB의 commit/rollback을 3진으로 완전 감싼다.

use std::collections::HashMap;
use std::time::Instant;

// ─────────────────────────────────────────────
// 트랜잭션 상태
// ─────────────────────────────────────────────

/// 트랜잭션 상태 (3진)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum TxState {
    Committed  =  1,  // P = 확정
    Pending    =  0,  // O = 보류 (진행중)
    RolledBack = -1,  // T = 취소 (복원됨)
}

impl std::fmt::Display for TxState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TxState::Committed => write!(f, "P(확정)"),
            TxState::Pending => write!(f, "O(보류)"),
            TxState::RolledBack => write!(f, "T(취소)"),
        }
    }
}

/// 합의 상태 (분산 환경용)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum TritConsensus {
    Approved =  1,  // P = 승인
    Holding  =  0,  // O = 보류
    Rejected = -1,  // T = 거부
}

impl std::fmt::Display for TritConsensus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TritConsensus::Approved => write!(f, "P(승인)"),
            TritConsensus::Holding => write!(f, "O(보류)"),
            TritConsensus::Rejected => write!(f, "T(거부)"),
        }
    }
}

// ─────────────────────────────────────────────
// WAL Entry (Write-Ahead Log)
// ─────────────────────────────────────────────

pub type TxId = u64;

/// 변경 기록 (WAL 엔트리)
#[derive(Debug, Clone)]
pub struct WalEntry {
    pub key: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

/// 트랜잭션
#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: TxId,
    pub state: TxState,
    pub wal: Vec<WalEntry>,        // 변경 로그
    pub created_at: Instant,
    pub finished_at: Option<Instant>,
    pub label: String,
}

impl Transaction {
    fn new(id: TxId, label: &str) -> Self {
        Self {
            id,
            state: TxState::Pending,
            wal: Vec::new(),
            created_at: Instant::now(),
            finished_at: None,
            label: label.to_string(),
        }
    }
}

impl std::fmt::Display for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TX[{:04}] '{}' {} (변경:{}건)", self.id, self.label, self.state, self.wal.len())
    }
}

// ─────────────────────────────────────────────
// Transaction Engine
// ─────────────────────────────────────────────

/// 균형3진 트랜잭션 엔진
pub struct TransactionEngine {
    /// 현재 데이터 저장소 (key→value)
    store: HashMap<String, String>,
    /// 활성 트랜잭션
    pub active: HashMap<TxId, Transaction>,
    /// 완료된 트랜잭션 이력
    history: Vec<Transaction>,
    /// 다음 TX ID
    next_id: TxId,
    /// 통계
    pub stats_commit: u64,
    pub stats_pending: u64,
    pub stats_rollback: u64,
}

impl TransactionEngine {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
            active: HashMap::new(),
            history: Vec::new(),
            next_id: 1,
            stats_commit: 0,
            stats_pending: 0,
            stats_rollback: 0,
        }
    }

    /// 트랜잭션 시작 → Pending(O) 상태
    pub fn begin(&mut self, label: &str) -> TxId {
        let id = self.next_id;
        self.next_id += 1;
        let tx = Transaction::new(id, label);
        self.active.insert(id, tx);
        self.stats_pending += 1;
        id
    }

    /// 트랜잭션 내에서 값 설정 (WAL에 기록)
    pub fn set(&mut self, tx_id: TxId, key: &str, value: &str) -> Result<(), String> {
        let tx = self.active.get_mut(&tx_id)
            .ok_or_else(|| format!("TX[{}] 존재하지 않음", tx_id))?;

        if tx.state != TxState::Pending {
            return Err(format!("TX[{}] 이미 완료됨: {}", tx_id, tx.state));
        }

        let old_value = self.store.get(key).cloned();
        tx.wal.push(WalEntry {
            key: key.to_string(),
            old_value,
            new_value: Some(value.to_string()),
        });

        // 즉시 적용 (optimistic)
        self.store.insert(key.to_string(), value.to_string());
        Ok(())
    }

    /// 트랜잭션 내에서 값 삭제
    pub fn delete(&mut self, tx_id: TxId, key: &str) -> Result<(), String> {
        let tx = self.active.get_mut(&tx_id)
            .ok_or_else(|| format!("TX[{}] 존재하지 않음", tx_id))?;

        if tx.state != TxState::Pending {
            return Err(format!("TX[{}] 이미 완료됨", tx_id));
        }

        let old_value = self.store.get(key).cloned();
        tx.wal.push(WalEntry {
            key: key.to_string(),
            old_value,
            new_value: None,  // 삭제
        });

        self.store.remove(key);
        Ok(())
    }

    /// 커밋 → Committed(P) 상태
    pub fn commit(&mut self, tx_id: TxId) -> Result<TxState, String> {
        let mut tx = self.active.remove(&tx_id)
            .ok_or_else(|| format!("TX[{}] 존재하지 않음", tx_id))?;

        tx.state = TxState::Committed;
        tx.finished_at = Some(Instant::now());
        self.stats_commit += 1;
        self.stats_pending -= 1;

        let state = tx.state;
        self.history.push(tx);
        Ok(state)
    }

    /// 롤백 → RolledBack(T) 상태
    /// WAL을 역순으로 되돌림
    pub fn rollback(&mut self, tx_id: TxId) -> Result<TxState, String> {
        let mut tx = self.active.remove(&tx_id)
            .ok_or_else(|| format!("TX[{}] 존재하지 않음", tx_id))?;

        // WAL 역순 적용 (undo)
        for entry in tx.wal.iter().rev() {
            match &entry.old_value {
                Some(old) => {
                    self.store.insert(entry.key.clone(), old.clone());
                }
                None => {
                    self.store.remove(&entry.key);
                }
            }
        }

        tx.state = TxState::RolledBack;
        tx.finished_at = Some(Instant::now());
        self.stats_rollback += 1;
        self.stats_pending -= 1;

        let state = tx.state;
        self.history.push(tx);
        Ok(state)
    }

    /// 값 읽기 (트랜잭션 외부에서도 가능)
    pub fn get(&self, key: &str) -> Option<&str> {
        self.store.get(key).map(|s| s.as_str())
    }

    /// 3진 합의 투표 (분산 트랜잭션용)
    /// votes: +1(승인), 0(보류), -1(거부)
    /// 결과: 다수결 (3진 median)
    pub fn consensus(votes: &[TritConsensus]) -> TritConsensus {
        if votes.is_empty() {
            return TritConsensus::Holding;
        }
        let sum: i32 = votes.iter().map(|v| *v as i8 as i32).sum();
        if sum > 0 { TritConsensus::Approved }
        else if sum < 0 { TritConsensus::Rejected }
        else { TritConsensus::Holding }
    }

    /// 활성 트랜잭션 수
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// 저장소 크기
    pub fn store_size(&self) -> usize {
        self.store.len()
    }

    /// 상태 덤프
    pub fn dump(&self) {
        println!("╔══ 트랜잭션 엔진 상태 ════════════════════╗");
        println!("║ 저장소: {} 키 | 활성TX: {} | 이력: {}",
            self.store.len(), self.active.len(), self.history.len());
        println!("║ 통계: 확정:{} 보류:{} 취소:{}",
            self.stats_commit, self.stats_pending, self.stats_rollback);

        if !self.active.is_empty() {
            println!("║ ── 활성 트랜잭션 ──");
            for (_, tx) in &self.active {
                println!("║   {}", tx);
            }
        }

        if !self.store.is_empty() {
            println!("║ ── 저장소 (최대 10) ──");
            for (i, (k, v)) in self.store.iter().enumerate().take(10) {
                println!("║   {} = {}", k, v);
            }
        }

        if !self.history.is_empty() {
            println!("║ ── 최근 이력 (최대 5) ──");
            for tx in self.history.iter().rev().take(5) {
                println!("║   {}", tx);
            }
        }
        println!("╚═══════════════════════════════════════════╝");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit() {
        let mut engine = TransactionEngine::new();

        let tx = engine.begin("테스트");
        engine.set(tx, "이름", "한선").unwrap();
        engine.set(tx, "버전", "1.0").unwrap();
        let state = engine.commit(tx).unwrap();

        assert_eq!(state, TxState::Committed);
        assert_eq!(engine.get("이름"), Some("한선"));
        assert_eq!(engine.get("버전"), Some("1.0"));
    }

    #[test]
    fn test_rollback() {
        let mut engine = TransactionEngine::new();

        // 초기값 설정
        let tx0 = engine.begin("초기화");
        engine.set(tx0, "값", "원래").unwrap();
        engine.commit(tx0).unwrap();

        // 변경 후 롤백
        let tx1 = engine.begin("변경");
        engine.set(tx1, "값", "새로운").unwrap();
        assert_eq!(engine.get("값"), Some("새로운"));

        let state = engine.rollback(tx1).unwrap();
        assert_eq!(state, TxState::RolledBack);
        assert_eq!(engine.get("값"), Some("원래")); // 복원됨
    }

    #[test]
    fn test_consensus() {
        use TritConsensus::*;
        // 2:1 → 승인
        assert_eq!(TransactionEngine::consensus(&[Approved, Approved, Rejected]), Approved);
        // 1:1:1 → 보류
        assert_eq!(TransactionEngine::consensus(&[Approved, Holding, Rejected]), Holding);
        // 0:1:2 → 거부
        assert_eq!(TransactionEngine::consensus(&[Holding, Rejected, Rejected]), Rejected);
    }
}
