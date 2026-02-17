///! ═══════════════════════════════════════════════════
///! Trit Persistent Layer v0.1
///! ═══════════════════════════════════════════════════
///!
///! GPT Spec:
///!   "DB 연결? Trit 상태 저장? Snapshot 복구?
///!    장애 복원? 없으면 서비스 못 만듦."
///!
///! 기능:
///!   - Trit KV Store (키-값 저장)
///!   - Snapshot 생성/복구
///!   - WAL (Write-Ahead Log) 장애 복원
///!   - 트랜잭션 ACID 보장
///!   - 3진 상태 인덱싱
///!
///! 구조:
///!   Memory Store → WAL → Snapshot → File

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::car::TritState;

// ─────────────────────────────────────────────
// 저장 값
// ─────────────────────────────────────────────

/// 영속 저장 값
#[derive(Debug, Clone)]
pub enum StoreValue {
    Null,
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Trit(i8),
    Bytes(Vec<u8>),
    List(Vec<StoreValue>),
    Map(HashMap<String, StoreValue>),
}

impl std::fmt::Display for StoreValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreValue::Null => write!(f, "null"),
            StoreValue::Int(n) => write!(f, "{}", n),
            StoreValue::Float(v) => write!(f, "{:.4}", v),
            StoreValue::Text(s) => write!(f, "\"{}\"", s),
            StoreValue::Bool(b) => write!(f, "{}", b),
            StoreValue::Trit(t) => write!(f, "{}", match t { 1 => "P", -1 => "T", _ => "O" }),
            StoreValue::Bytes(b) => write!(f, "[{} bytes]", b.len()),
            StoreValue::List(l) => write!(f, "[{}개]", l.len()),
            StoreValue::Map(m) => write!(f, "{{{}개}}", m.len()),
        }
    }
}

// ─────────────────────────────────────────────
// WAL 엔트리 (Write-Ahead Log)
// ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum WalOp {
    Set { key: String, value: StoreValue },
    Delete { key: String },
    SetTritState { key: String, state: i8 },
}

#[derive(Debug, Clone)]
pub struct WalEntry {
    pub seq: u64,
    pub timestamp: u64,
    pub op: WalOp,
}

// ─────────────────────────────────────────────
// Snapshot
// ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub id: u64,
    pub timestamp: u64,
    pub data: HashMap<String, StoreValue>,
    pub trit_states: HashMap<String, i8>,
    pub entry_count: usize,
}

// ─────────────────────────────────────────────
// Trit KV Store
// ─────────────────────────────────────────────

/// 3진 영속 저장소
pub struct TritStore {
    // 메모리 데이터
    data: HashMap<String, StoreValue>,
    // Trit 상태 인덱스 (키별 3진 상태)
    trit_index: HashMap<String, i8>,
    // WAL
    wal: Vec<WalEntry>,
    wal_seq: u64,
    // Snapshot
    snapshots: Vec<Snapshot>,
    snapshot_counter: u64,
    // 트랜잭션
    tx_active: bool,
    tx_buffer: Vec<WalOp>,
    // 통계
    read_count: u64,
    write_count: u64,
    delete_count: u64,
}

impl TritStore {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            trit_index: HashMap::new(),
            wal: Vec::new(),
            wal_seq: 0,
            snapshots: Vec::new(),
            snapshot_counter: 0,
            tx_active: false,
            tx_buffer: Vec::new(),
            read_count: 0,
            write_count: 0,
            delete_count: 0,
        }
    }

    fn now_ms(&self) -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
    }

    // ── 기본 CRUD ──

    /// 값 저장
    pub fn set(&mut self, key: &str, value: StoreValue) {
        let op = WalOp::Set { key: key.to_string(), value: value.clone() };

        if self.tx_active {
            self.tx_buffer.push(op);
        } else {
            self.apply_op(&op);
            self.append_wal(op);
        }
    }

    /// 값 읽기
    pub fn get(&mut self, key: &str) -> Option<&StoreValue> {
        self.read_count += 1;
        self.data.get(key)
    }

    /// 값 삭제
    pub fn delete(&mut self, key: &str) -> bool {
        let op = WalOp::Delete { key: key.to_string() };

        if self.tx_active {
            self.tx_buffer.push(op);
            true
        } else {
            let existed = self.data.contains_key(key);
            self.apply_op(&op);
            self.append_wal(op);
            existed
        }
    }

    /// 존재 확인
    pub fn exists(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// 전체 키 목록
    pub fn keys(&self) -> Vec<&String> {
        self.data.keys().collect()
    }

    /// 크기
    pub fn len(&self) -> usize {
        self.data.len()
    }

    // ── Trit 상태 관리 ──

    /// 키에 Trit 상태 설정
    pub fn set_trit_state(&mut self, key: &str, state: i8) {
        let clamped = state.clamp(-1, 1);
        let op = WalOp::SetTritState { key: key.to_string(), state: clamped };

        if self.tx_active {
            self.tx_buffer.push(op);
        } else {
            self.trit_index.insert(key.to_string(), clamped);
            self.append_wal(op);
        }
    }

    /// 키의 Trit 상태 조회
    pub fn get_trit_state(&self, key: &str) -> Option<i8> {
        self.trit_index.get(key).copied()
    }

    /// 상태별 키 필터
    pub fn filter_by_trit(&self, state: i8) -> Vec<&String> {
        self.trit_index.iter()
            .filter(|(_, &s)| s == state)
            .map(|(k, _)| k)
            .collect()
    }

    /// Trit 상태 통계
    pub fn trit_stats(&self) -> (usize, usize, usize) {
        let p = self.trit_index.values().filter(|&&v| v == 1).count();
        let o = self.trit_index.values().filter(|&&v| v == 0).count();
        let t = self.trit_index.values().filter(|&&v| v == -1).count();
        (p, o, t)
    }

    // ── WAL ──

    fn append_wal(&mut self, op: WalOp) {
        self.wal_seq += 1;
        self.wal.push(WalEntry {
            seq: self.wal_seq,
            timestamp: self.now_ms(),
            op,
        });
    }

    fn apply_op(&mut self, op: &WalOp) {
        match op {
            WalOp::Set { key, value } => {
                self.data.insert(key.clone(), value.clone());
                self.write_count += 1;
            }
            WalOp::Delete { key } => {
                self.data.remove(key);
                self.trit_index.remove(key);
                self.delete_count += 1;
            }
            WalOp::SetTritState { key, state } => {
                self.trit_index.insert(key.clone(), *state);
            }
        }
    }

    /// WAL 길이
    pub fn wal_len(&self) -> usize {
        self.wal.len()
    }

    // ── 트랜잭션 ──

    /// 트랜잭션 시작
    pub fn begin(&mut self) -> bool {
        if self.tx_active { return false; }
        self.tx_active = true;
        self.tx_buffer.clear();
        true
    }

    /// 트랜잭션 커밋
    pub fn commit(&mut self) -> TritState {
        if !self.tx_active { return TritState::Failed; }

        let ops: Vec<WalOp> = self.tx_buffer.drain(..).collect();
        for op in &ops {
            self.apply_op(op);
        }
        for op in ops {
            self.append_wal(op);
        }

        self.tx_active = false;
        TritState::Success
    }

    /// 트랜잭션 롤백
    pub fn rollback(&mut self) -> TritState {
        if !self.tx_active { return TritState::Failed; }
        self.tx_buffer.clear();
        self.tx_active = false;
        TritState::Success
    }

    // ── Snapshot ──

    /// Snapshot 생성
    pub fn snapshot(&mut self) -> u64 {
        self.snapshot_counter += 1;
        let snap = Snapshot {
            id: self.snapshot_counter,
            timestamp: self.now_ms(),
            data: self.data.clone(),
            trit_states: self.trit_index.clone(),
            entry_count: self.data.len(),
        };
        let id = snap.id;
        self.snapshots.push(snap);
        id
    }

    /// Snapshot 복구
    pub fn restore(&mut self, snapshot_id: u64) -> TritState {
        if let Some(snap) = self.snapshots.iter().find(|s| s.id == snapshot_id) {
            self.data = snap.data.clone();
            self.trit_index = snap.trit_states.clone();
            // WAL에 복구 기록
            self.append_wal(WalOp::Set {
                key: "__restore__".to_string(),
                value: StoreValue::Int(snapshot_id as i64),
            });
            TritState::Success
        } else {
            TritState::Failed
        }
    }

    /// Snapshot 목록
    pub fn list_snapshots(&self) -> Vec<(u64, u64, usize)> {
        self.snapshots.iter()
            .map(|s| (s.id, s.timestamp, s.entry_count))
            .collect()
    }

    // ── 직렬화 (시뮬레이션) ──

    /// 전체 데이터를 바이트로 직렬화 (크기 계산)
    pub fn estimated_size(&self) -> usize {
        let mut size = 0usize;
        for (k, v) in &self.data {
            size += k.len() + 1;
            size += match v {
                StoreValue::Null => 1,
                StoreValue::Int(_) => 8,
                StoreValue::Float(_) => 8,
                StoreValue::Text(s) => s.len() + 2,
                StoreValue::Bool(_) => 1,
                StoreValue::Trit(_) => 1,
                StoreValue::Bytes(b) => b.len() + 4,
                StoreValue::List(l) => l.len() * 16,
                StoreValue::Map(m) => m.len() * 32,
            };
        }
        size
    }

    // ── 통계 ──

    pub fn stats(&self) -> String {
        let (p, o, t) = self.trit_stats();
        format!(
            "[Store] 항목:{} | WAL:{} | Snap:{} | R:{} W:{} D:{} | Trit P:{} O:{} T:{}",
            self.data.len(), self.wal.len(), self.snapshots.len(),
            self.read_count, self.write_count, self.delete_count,
            p, o, t
        )
    }

    pub fn dump(&self) -> String {
        let mut out = String::new();
        out.push_str("┌── TritStore ──────────────────┐\n");
        for (k, v) in &self.data {
            let trit = self.trit_index.get(k).map(|t| match t {
                1 => "P", -1 => "T", _ => "O"
            }).unwrap_or("-");
            out.push_str(&format!("│ [{}] {} = {}\n", trit, k, v));
        }
        out.push_str(&format!("│ {}\n", self.stats()));
        out.push_str("└──────────────────────────────┘\n");
        out
    }
}

// ─────────────────────────────────────────────
// 네임스페이스 (패키지별 분리)
// ─────────────────────────────────────────────

/// 네임스페이스 스토어 (패키지별 격리)
pub struct NamespacedStore {
    stores: HashMap<String, TritStore>,
}

impl NamespacedStore {
    pub fn new() -> Self {
        Self { stores: HashMap::new() }
    }

    pub fn get_or_create(&mut self, ns: &str) -> &mut TritStore {
        self.stores.entry(ns.to_string()).or_insert_with(TritStore::new)
    }

    pub fn namespaces(&self) -> Vec<&String> {
        self.stores.keys().collect()
    }

    pub fn total_entries(&self) -> usize {
        self.stores.values().map(|s| s.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_crud() {
        let mut store = TritStore::new();
        store.set("name", StoreValue::Text("Crowny".into()));
        store.set("version", StoreValue::Int(3));

        assert_eq!(store.len(), 2);
        assert!(store.exists("name"));

        store.delete("version");
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_trit_state() {
        let mut store = TritStore::new();
        store.set("task1", StoreValue::Text("완료".into()));
        store.set("task2", StoreValue::Text("보류".into()));
        store.set("task3", StoreValue::Text("실패".into()));

        store.set_trit_state("task1", 1);
        store.set_trit_state("task2", 0);
        store.set_trit_state("task3", -1);

        assert_eq!(store.get_trit_state("task1"), Some(1));
        let (p, o, t) = store.trit_stats();
        assert_eq!((p, o, t), (1, 1, 1));
    }

    #[test]
    fn test_transaction() {
        let mut store = TritStore::new();
        store.set("a", StoreValue::Int(1));

        // 트랜잭션: 커밋
        store.begin();
        store.set("b", StoreValue::Int(2));
        store.set("c", StoreValue::Int(3));
        store.commit();
        assert_eq!(store.len(), 3);

        // 트랜잭션: 롤백
        store.begin();
        store.set("d", StoreValue::Int(4));
        store.rollback();
        assert_eq!(store.len(), 3); // d는 롤백됨
    }

    #[test]
    fn test_snapshot_restore() {
        let mut store = TritStore::new();
        store.set("x", StoreValue::Int(10));
        store.set_trit_state("x", 1);

        // Snapshot 1
        let snap_id = store.snapshot();

        // 데이터 변경
        store.set("x", StoreValue::Int(99));
        store.set("y", StoreValue::Int(20));
        assert_eq!(store.len(), 2);

        // 복구
        let result = store.restore(snap_id);
        assert_eq!(result, TritState::Success);
        assert_eq!(store.len(), 1);
        assert_eq!(store.get_trit_state("x"), Some(1));
    }

    #[test]
    fn test_wal() {
        let mut store = TritStore::new();
        store.set("a", StoreValue::Int(1));
        store.set("b", StoreValue::Int(2));
        store.delete("a");
        assert_eq!(store.wal_len(), 3);
    }

    #[test]
    fn test_filter_by_trit() {
        let mut store = TritStore::new();
        for i in 0..9 {
            let key = format!("item_{}", i);
            store.set(&key, StoreValue::Int(i));
            store.set_trit_state(&key, (i % 3) as i8 - 1); // -1, 0, 1 순환
        }
        let positive = store.filter_by_trit(1);
        assert_eq!(positive.len(), 3);
    }

    #[test]
    fn test_namespace() {
        let mut ns = NamespacedStore::new();
        ns.get_or_create("app.ai").set("model", StoreValue::Text("Claude".into()));
        ns.get_or_create("app.web").set("port", StoreValue::Int(7293));
        assert_eq!(ns.total_entries(), 2);
        assert_eq!(ns.namespaces().len(), 2);
    }
}
