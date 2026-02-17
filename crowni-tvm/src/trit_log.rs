///! ═══════════════════════════════════════════════════
///! Trit Event Log v0.1 — 관측성 (Observability)
///! ═══════════════════════════════════════════════════
///!
///! GPT Spec:
///!   "Task 추적, Trit 상태 흐름 기록, 합의 기록,
///!    오류 분석. 없으면 운영 불가."
///!
///! 기능:
///!   - 이벤트 스트림 (구조화 로그)
///!   - Trit 상태 전이 추적
///!   - Task 라이프사이클 추적
///!   - 합의 과정 기록
///!   - 권한 감사 로그
///!   - 메트릭 수집 (카운터/게이지/히스토그램)
///!   - 알림 규칙 (임계치 초과 시)
///!
///! 모든 이벤트는 TritState 포함.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH, Instant};
use crate::car::TritState;

// ─────────────────────────────────────────────
// 이벤트
// ─────────────────────────────────────────────

/// 로그 레벨
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Level {
    Trace = 0,
    Debug = 1,
    Info  = 2,
    Warn  = 3,
    Error = 4,
    Fatal = 5,
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Level::Trace => write!(f, "TRACE"),
            Level::Debug => write!(f, "DEBUG"),
            Level::Info  => write!(f, "INFO"),
            Level::Warn  => write!(f, "WARN"),
            Level::Error => write!(f, "ERROR"),
            Level::Fatal => write!(f, "FATAL"),
        }
    }
}

/// 이벤트 카테고리
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Category {
    Task,       // Task 실행
    State,      // 상태 전이
    Consensus,  // 합의
    Permission, // 권한
    Network,    // 네트워크
    Store,      // 영속화
    Llm,        // LLM 호출
    System,     // 시스템
    User,       // 사용자 정의
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Category::Task => write!(f, "TASK"),
            Category::State => write!(f, "STATE"),
            Category::Consensus => write!(f, "CONS"),
            Category::Permission => write!(f, "PERM"),
            Category::Network => write!(f, "NET"),
            Category::Store => write!(f, "STORE"),
            Category::Llm => write!(f, "LLM"),
            Category::System => write!(f, "SYS"),
            Category::User => write!(f, "USER"),
        }
    }
}

/// 구조화 이벤트
#[derive(Debug, Clone)]
pub struct Event {
    pub id: u64,
    pub timestamp: u64,
    pub level: Level,
    pub category: Category,
    pub trit_state: TritState,
    pub source: String,
    pub message: String,
    pub fields: HashMap<String, String>,
}

impl Event {
    /// 로그 포맷 출력
    pub fn format(&self) -> String {
        let trit_ch = match self.trit_state {
            TritState::Success => "P",
            TritState::Pending => "O",
            TritState::Failed => "T",
        };
        let fields_str = if self.fields.is_empty() {
            String::new()
        } else {
            let pairs: Vec<String> = self.fields.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            format!(" {}", pairs.join(" "))
        };
        format!("[{}] {} [{}] {} | {} — {}{}",
            self.timestamp % 100000, // 마지막 5자리
            self.level, trit_ch, self.category,
            self.source, self.message, fields_str)
    }
}

// ─────────────────────────────────────────────
// 이벤트 빌더
// ─────────────────────────────────────────────

pub struct EventBuilder {
    level: Level,
    category: Category,
    trit_state: TritState,
    source: String,
    message: String,
    fields: HashMap<String, String>,
}

impl EventBuilder {
    pub fn new(category: Category, message: &str) -> Self {
        Self {
            level: Level::Info,
            category,
            trit_state: TritState::Pending,
            source: String::new(),
            message: message.to_string(),
            fields: HashMap::new(),
        }
    }

    pub fn level(mut self, level: Level) -> Self { self.level = level; self }
    pub fn trit(mut self, state: TritState) -> Self { self.trit_state = state; self }
    pub fn source(mut self, src: &str) -> Self { self.source = src.to_string(); self }
    pub fn field(mut self, key: &str, val: &str) -> Self {
        self.fields.insert(key.to_string(), val.to_string()); self
    }

    fn build(self, id: u64, timestamp: u64) -> Event {
        Event {
            id, timestamp,
            level: self.level,
            category: self.category,
            trit_state: self.trit_state,
            source: self.source,
            message: self.message,
            fields: self.fields,
        }
    }
}

// ─────────────────────────────────────────────
// 메트릭
// ─────────────────────────────────────────────

/// 메트릭 종류
#[derive(Debug, Clone)]
pub enum Metric {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
}

impl std::fmt::Display for Metric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Metric::Counter(n) => write!(f, "{}", n),
            Metric::Gauge(v) => write!(f, "{:.2}", v),
            Metric::Histogram(vals) => {
                let avg = vals.iter().sum::<f64>() / vals.len().max(1) as f64;
                write!(f, "avg={:.2} n={}", avg, vals.len())
            }
        }
    }
}

// ─────────────────────────────────────────────
// 알림 규칙
// ─────────────────────────────────────────────

/// 알림 조건
pub struct AlertRule {
    pub name: String,
    pub category: Category,
    pub min_level: Level,
    pub trit_filter: Option<TritState>,
    pub triggered_count: u64,
}

impl AlertRule {
    pub fn new(name: &str, category: Category, min_level: Level) -> Self {
        Self {
            name: name.to_string(),
            category,
            min_level,
            trit_filter: None,
            triggered_count: 0,
        }
    }

    pub fn with_trit(mut self, state: TritState) -> Self {
        self.trit_filter = Some(state); self
    }

    fn matches(&self, event: &Event) -> bool {
        event.category == self.category
            && event.level >= self.min_level
            && self.trit_filter.map_or(true, |s| event.trit_state == s)
    }
}

// ─────────────────────────────────────────────
// Trit Event Logger
// ─────────────────────────────────────────────

pub struct TritEventLog {
    events: Vec<Event>,
    event_counter: u64,
    start_time: Instant,
    // 메트릭
    metrics: HashMap<String, Metric>,
    // 알림
    alerts: Vec<AlertRule>,
    alert_log: Vec<(String, u64)>,  // (규칙명, 이벤트ID)
    // 설정
    min_level: Level,
    max_events: usize,
    // 카테고리별 카운트
    category_counts: HashMap<String, u64>,
    trit_counts: [u64; 3], // [T, O, P]
}

impl TritEventLog {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            event_counter: 0,
            start_time: Instant::now(),
            metrics: HashMap::new(),
            alerts: Vec::new(),
            alert_log: Vec::new(),
            min_level: Level::Info,
            max_events: 10000,
            category_counts: HashMap::new(),
            trit_counts: [0; 3],
        }
    }

    pub fn set_min_level(&mut self, level: Level) {
        self.min_level = level;
    }

    pub fn add_alert(&mut self, rule: AlertRule) {
        self.alerts.push(rule);
    }

    fn now_ms(&self) -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
    }

    /// 이벤트 기록
    pub fn log(&mut self, builder: EventBuilder) {
        self.event_counter += 1;
        let event = builder.build(self.event_counter, self.now_ms());

        // 레벨 필터
        if event.level < self.min_level { return; }

        // 카테고리 카운트
        *self.category_counts.entry(event.category.to_string()).or_insert(0) += 1;

        // Trit 카운트
        match event.trit_state {
            TritState::Failed => self.trit_counts[0] += 1,
            TritState::Pending => self.trit_counts[1] += 1,
            TritState::Success => self.trit_counts[2] += 1,
        }

        // 알림 체크
        for alert in &mut self.alerts {
            if alert.matches(&event) {
                alert.triggered_count += 1;
                // alert_log는 나중에 처리
            }
        }
        // 알림 로그 별도 수집
        let triggered: Vec<String> = self.alerts.iter()
            .filter(|a| a.matches(&event))
            .map(|a| a.name.clone())
            .collect();
        for name in triggered {
            self.alert_log.push((name, event.id));
        }

        // 용량 제한
        if self.events.len() >= self.max_events {
            self.events.drain(0..self.max_events / 4); // 25% 정리
        }

        self.events.push(event);
    }

    // ── 편의 메서드 ──

    pub fn info(&mut self, cat: Category, src: &str, msg: &str, state: TritState) {
        self.log(EventBuilder::new(cat, msg).level(Level::Info).source(src).trit(state));
    }

    pub fn warn(&mut self, cat: Category, src: &str, msg: &str) {
        self.log(EventBuilder::new(cat, msg).level(Level::Warn).source(src).trit(TritState::Pending));
    }

    pub fn error(&mut self, cat: Category, src: &str, msg: &str) {
        self.log(EventBuilder::new(cat, msg).level(Level::Error).source(src).trit(TritState::Failed));
    }

    pub fn task_start(&mut self, task_id: u64, subject: &str) {
        self.log(EventBuilder::new(Category::Task, &format!("Task#{} 시작: {}", task_id, subject))
            .level(Level::Info).source("CAR").trit(TritState::Pending)
            .field("task_id", &task_id.to_string()));
    }

    pub fn task_end(&mut self, task_id: u64, state: TritState) {
        self.log(EventBuilder::new(Category::Task, &format!("Task#{} 완료", task_id))
            .level(Level::Info).source("CAR").trit(state)
            .field("task_id", &task_id.to_string()));
    }

    pub fn state_transition(&mut self, key: &str, from: i8, to: i8) {
        let trit_ch = |v: i8| match v { 1 => "P", -1 => "T", _ => "O" };
        self.log(EventBuilder::new(Category::State,
            &format!("{}: {} → {}", key, trit_ch(from), trit_ch(to)))
            .level(Level::Debug).source("TritState")
            .trit(match to { 1 => TritState::Success, -1 => TritState::Failed, _ => TritState::Pending })
            .field("from", trit_ch(from)).field("to", trit_ch(to)));
    }

    pub fn consensus_vote(&mut self, round: u32, voter: &str, vote: i8) {
        let trit_ch = |v: i8| match v { 1 => "P", -1 => "T", _ => "O" };
        self.log(EventBuilder::new(Category::Consensus,
            &format!("Round#{} {} 투표: {}", round, voter, trit_ch(vote)))
            .level(Level::Info).source("Consensus")
            .trit(match vote { 1 => TritState::Success, -1 => TritState::Failed, _ => TritState::Pending }));
    }

    pub fn permission_check(&mut self, subject: &str, resource: &str, allowed: bool) {
        let state = if allowed { TritState::Success } else { TritState::Failed };
        let level = if allowed { Level::Debug } else { Level::Warn };
        self.log(EventBuilder::new(Category::Permission,
            &format!("{} → {} : {}", subject, resource, if allowed { "허용" } else { "거부" }))
            .level(level).source("Permission").trit(state));
    }

    // ── 메트릭 ──

    pub fn increment(&mut self, name: &str) {
        let metric = self.metrics.entry(name.to_string())
            .or_insert(Metric::Counter(0));
        if let Metric::Counter(ref mut n) = metric { *n += 1; }
    }

    pub fn gauge(&mut self, name: &str, value: f64) {
        self.metrics.insert(name.to_string(), Metric::Gauge(value));
    }

    pub fn record(&mut self, name: &str, value: f64) {
        let metric = self.metrics.entry(name.to_string())
            .or_insert(Metric::Histogram(Vec::new()));
        if let Metric::Histogram(ref mut vals) = metric { vals.push(value); }
    }

    // ── 조회 ──

    /// 최근 N개 이벤트
    pub fn recent(&self, n: usize) -> &[Event] {
        let start = self.events.len().saturating_sub(n);
        &self.events[start..]
    }

    /// 카테고리 필터
    pub fn filter_category(&self, cat: &Category) -> Vec<&Event> {
        self.events.iter().filter(|e| &e.category == cat).collect()
    }

    /// Trit 상태 필터
    pub fn filter_trit(&self, state: TritState) -> Vec<&Event> {
        self.events.iter().filter(|e| e.trit_state == state).collect()
    }

    /// 에러만
    pub fn errors(&self) -> Vec<&Event> {
        self.events.iter().filter(|e| e.level >= Level::Error).collect()
    }

    /// 전체 이벤트 수
    pub fn total_events(&self) -> u64 {
        self.event_counter
    }

    // ── 보고서 ──

    pub fn summary(&self) -> String {
        let elapsed = self.start_time.elapsed().as_secs();
        let mut out = String::new();
        out.push_str("╔══ Trit Event Log 요약 ════════════════╗\n");
        out.push_str(&format!("║ 총 이벤트: {} | 가동: {}s\n", self.event_counter, elapsed));
        out.push_str(&format!("║ Trit: P:{} O:{} T:{}\n",
            self.trit_counts[2], self.trit_counts[1], self.trit_counts[0]));

        // 카테고리별
        if !self.category_counts.is_empty() {
            out.push_str("║ ─── 카테고리별 ───\n");
            let mut sorted: Vec<_> = self.category_counts.iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(a.1));
            for (cat, count) in sorted {
                out.push_str(&format!("║   {}: {}\n", cat, count));
            }
        }

        // 메트릭
        if !self.metrics.is_empty() {
            out.push_str("║ ─── 메트릭 ───\n");
            for (name, metric) in &self.metrics {
                out.push_str(&format!("║   {}: {}\n", name, metric));
            }
        }

        // 알림
        if !self.alert_log.is_empty() {
            out.push_str(&format!("║ ─── 알림: {}건 ───\n", self.alert_log.len()));
            for (name, eid) in self.alert_log.iter().rev().take(5) {
                out.push_str(&format!("║   ⚠ {} (Event#{})\n", name, eid));
            }
        }

        out.push_str("╚═══════════════════════════════════════╝\n");
        out
    }

    pub fn dump_recent(&self, n: usize) -> String {
        let mut out = String::new();
        out.push_str("┌── 최근 이벤트 ────────────────┐\n");
        for event in self.recent(n) {
            out.push_str(&format!("│ {}\n", event.format()));
        }
        out.push_str("└──────────────────────────────┘\n");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_logging() {
        let mut log = TritEventLog::new();
        log.info(Category::System, "test", "시스템 시작", TritState::Success);
        log.info(Category::Task, "test", "작업 실행", TritState::Pending);
        log.error(Category::Task, "test", "작업 실패");
        assert_eq!(log.total_events(), 3);
    }

    #[test]
    fn test_trit_counts() {
        let mut log = TritEventLog::new();
        log.info(Category::Task, "t", "a", TritState::Success);
        log.info(Category::Task, "t", "b", TritState::Success);
        log.info(Category::Task, "t", "c", TritState::Failed);
        log.info(Category::Task, "t", "d", TritState::Pending);

        let errors = log.filter_trit(TritState::Failed);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_task_lifecycle() {
        let mut log = TritEventLog::new();
        log.task_start(1, "컴파일");
        log.task_end(1, TritState::Success);

        let tasks = log.filter_category(&Category::Task);
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_consensus_logging() {
        let mut log = TritEventLog::new();
        log.consensus_vote(1, "Claude", 1);
        log.consensus_vote(1, "GPT-4", 1);
        log.consensus_vote(1, "Gemini", -1);

        let cons = log.filter_category(&Category::Consensus);
        assert_eq!(cons.len(), 3);
    }

    #[test]
    fn test_metrics() {
        let mut log = TritEventLog::new();
        log.increment("requests");
        log.increment("requests");
        log.increment("requests");
        log.gauge("cpu_usage", 45.2);
        log.record("latency_ms", 12.5);
        log.record("latency_ms", 8.3);

        assert_eq!(log.metrics.len(), 3);
    }

    #[test]
    fn test_alert() {
        let mut log = TritEventLog::new();
        log.add_alert(AlertRule::new("에러감지", Category::Task, Level::Error));

        log.info(Category::Task, "t", "정상", TritState::Success);
        log.error(Category::Task, "t", "실패!");
        log.error(Category::Task, "t", "또 실패!");

        assert_eq!(log.alert_log.len(), 2);
    }

    #[test]
    fn test_permission_audit() {
        let mut log = TritEventLog::new();
        log.set_min_level(Level::Debug); // Debug 레벨 허용
        log.permission_check("user:kim", "kernel:execute", true);
        log.permission_check("guest", "admin:shutdown", false);

        let perms = log.filter_category(&Category::Permission);
        assert_eq!(perms.len(), 2);
    }
}
