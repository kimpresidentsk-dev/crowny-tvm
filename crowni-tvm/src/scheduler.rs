///! ═══════════════════════════════════════════════════
///! Trit Scheduler — 균형3진 스케줄러
///! ═══════════════════════════════════════════════════
///!
///! 모든 상태가 3진:
///!   Task 상태:  P=활성, O=중립(대기), T=비활성(취소)
///!   우선순위:   P=높음, O=보통, T=낮음
///!   실행 결과:  P=성공, O=보류, T=실패
///!
///! 2진 OS의 스레드/프로세스 개념을 3진 논리로 감싼다.
///! 절대 2진 상태를 노출하지 않는다.

use std::collections::VecDeque;
use std::time::{Instant, Duration};

// ─────────────────────────────────────────────
// 3진 상태 타입들
// ─────────────────────────────────────────────

/// 3진 상태: 모든 Crowny 시스템의 기본 상태 모델
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum TritState {
    Active   =  1,  // 티(P) = 활성
    Neutral  =  0,  // 옴(O) = 중립/대기
    Inactive = -1,  // 타(T) = 비활성/취소
}

impl TritState {
    pub fn symbol(&self) -> char {
        match self {
            TritState::Active => 'P',
            TritState::Neutral => 'O',
            TritState::Inactive => 'T',
        }
    }

    pub fn name_kr(&self) -> &'static str {
        match self {
            TritState::Active => "활성",
            TritState::Neutral => "중립",
            TritState::Inactive => "비활성",
        }
    }
}

impl std::fmt::Display for TritState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.symbol(), self.name_kr())
    }
}

/// 3진 우선순위
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i8)]
pub enum TritPriority {
    High   =  1,  // P = 높음
    Normal =  0,  // O = 보통
    Low    = -1,  // T = 낮음
}

impl TritPriority {
    pub fn name_kr(&self) -> &'static str {
        match self {
            TritPriority::High => "높음",
            TritPriority::Normal => "보통",
            TritPriority::Low => "낮음",
        }
    }
}

impl std::fmt::Display for TritPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name_kr())
    }
}

/// 실행 결과 (3진)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum TritResult {
    Success =  1,  // P = 성공
    Pending =  0,  // O = 보류
    Failed  = -1,  // T = 실패
}

impl std::fmt::Display for TritResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TritResult::Success => write!(f, "P(성공)"),
            TritResult::Pending => write!(f, "O(보류)"),
            TritResult::Failed => write!(f, "T(실패)"),
        }
    }
}

// ─────────────────────────────────────────────
// Task
// ─────────────────────────────────────────────

/// 태스크 ID
pub type TaskId = u64;

/// 태스크 콜백 타입
pub type TaskFn = Box<dyn FnOnce() -> TritResult + Send>;

/// 스케줄러 태스크
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub state: TritState,
    pub priority: TritPriority,
    pub result: TritResult,
    pub created_at: Instant,
    pub started_at: Option<Instant>,
    pub finished_at: Option<Instant>,
    pub action: Option<TaskFn>,
    /// 재시도 카운터 (3진: 최대 3회)
    pub retries: u8,
    pub max_retries: u8,
}

impl Task {
    pub fn new(id: TaskId, name: &str, priority: TritPriority, action: TaskFn) -> Self {
        Self {
            id,
            name: name.to_string(),
            state: TritState::Neutral,  // 생성 시 대기 상태
            priority,
            result: TritResult::Pending,
            created_at: Instant::now(),
            started_at: None,
            finished_at: None,
            action: Some(action),
            retries: 0,
            max_retries: 3,  // 3진답게 최대 3회
        }
    }

    /// 경과 시간
    pub fn elapsed(&self) -> Duration {
        if let Some(end) = self.finished_at {
            if let Some(start) = self.started_at {
                return end.duration_since(start);
            }
        }
        if let Some(start) = self.started_at {
            return start.elapsed();
        }
        self.created_at.elapsed()
    }
}

impl std::fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Task[{}] '{}' 상태:{} 우선:{} 결과:{}",
            self.id, self.name, self.state, self.priority, self.result)
    }
}

// ─────────────────────────────────────────────
// TritScheduler
// ─────────────────────────────────────────────

/// 균형3진 스케줄러
/// 3개 큐: 높음(P), 보통(O), 낮음(T)
pub struct TritScheduler {
    /// 우선순위별 태스크 큐 (P/O/T 3개)
    queue_high: VecDeque<Task>,
    queue_normal: VecDeque<Task>,
    queue_low: VecDeque<Task>,
    /// 완료된 태스크 기록
    completed: Vec<Task>,
    /// 다음 태스크 ID
    next_id: TaskId,
    /// 총 실행 횟수
    pub total_executed: u64,
    /// 총 성공/보류/실패 카운트 (3진 통계)
    pub stats_success: u64,
    pub stats_pending: u64,
    pub stats_failed: u64,
}

impl TritScheduler {
    pub fn new() -> Self {
        Self {
            queue_high: VecDeque::new(),
            queue_normal: VecDeque::new(),
            queue_low: VecDeque::new(),
            completed: Vec::new(),
            next_id: 1,
            total_executed: 0,
            stats_success: 0,
            stats_pending: 0,
            stats_failed: 0,
        }
    }

    /// 태스크 등록 (큐에 넣기)
    pub fn submit(&mut self, name: &str, priority: TritPriority, action: TaskFn) -> TaskId {
        let id = self.next_id;
        self.next_id += 1;
        let task = Task::new(id, name, priority, action);

        match priority {
            TritPriority::High => self.queue_high.push_back(task),
            TritPriority::Normal => self.queue_normal.push_back(task),
            TritPriority::Low => self.queue_low.push_back(task),
        }

        id
    }

    /// 다음 태스크 꺼내기 (우선순위 순: P → O → T)
    fn dequeue(&mut self) -> Option<Task> {
        if let Some(t) = self.queue_high.pop_front() { return Some(t); }
        if let Some(t) = self.queue_normal.pop_front() { return Some(t); }
        if let Some(t) = self.queue_low.pop_front() { return Some(t); }
        None
    }

    /// 단일 태스크 실행
    pub fn execute_one(&mut self) -> Option<(TaskId, TritResult)> {
        let mut task = self.dequeue()?;

        // 비활성(취소) 상태면 건너뜀
        if task.state == TritState::Inactive {
            task.result = TritResult::Failed;
            task.finished_at = Some(Instant::now());
            let id = task.id;
            self.completed.push(task);
            self.stats_failed += 1;
            return Some((id, TritResult::Failed));
        }

        // 활성화
        task.state = TritState::Active;
        task.started_at = Some(Instant::now());

        // 실행
        let result = if let Some(action) = task.action.take() {
            action()
        } else {
            TritResult::Failed
        };

        task.result = result;
        task.finished_at = Some(Instant::now());
        self.total_executed += 1;

        // 결과 처리
        match result {
            TritResult::Success => {
                task.state = TritState::Inactive; // 완료
                self.stats_success += 1;
            }
            TritResult::Pending => {
                // 보류 → 재시도 가능하면 다시 큐에
                if task.retries < task.max_retries {
                    task.retries += 1;
                    task.state = TritState::Neutral;
                    self.stats_pending += 1;
                    let id = task.id;
                    // 재큐잉 (우선순위 한 단계 낮춤)
                    match task.priority {
                        TritPriority::High => self.queue_normal.push_back(task),
                        _ => self.queue_low.push_back(task),
                    }
                    return Some((id, TritResult::Pending));
                } else {
                    task.state = TritState::Inactive;
                    self.stats_failed += 1;
                }
            }
            TritResult::Failed => {
                task.state = TritState::Inactive;
                self.stats_failed += 1;
            }
        }

        let id = task.id;
        let res = task.result;
        self.completed.push(task);
        Some((id, res))
    }

    /// 모든 대기 태스크 실행
    pub fn run_all(&mut self) -> Vec<(TaskId, TritResult)> {
        let mut results = Vec::new();
        while let Some(r) = self.execute_one() {
            results.push(r);
        }
        results
    }

    /// 대기 중인 태스크 수
    pub fn pending_count(&self) -> usize {
        self.queue_high.len() + self.queue_normal.len() + self.queue_low.len()
    }

    /// 태스크 취소 (상태를 T로 전환)
    pub fn cancel(&mut self, id: TaskId) -> bool {
        for q in [&mut self.queue_high, &mut self.queue_normal, &mut self.queue_low] {
            for task in q.iter_mut() {
                if task.id == id {
                    task.state = TritState::Inactive;
                    return true;
                }
            }
        }
        false
    }

    /// 상태 덤프
    pub fn dump(&self) {
        println!("╔══ 스케줄러 상태 ══════════════════════════╗");
        println!("║ 대기: P:{} O:{} T:{}  완료:{}",
            self.queue_high.len(), self.queue_normal.len(),
            self.queue_low.len(), self.completed.len());
        println!("║ 통계: 성공:{} 보류:{} 실패:{} 총:{}",
            self.stats_success, self.stats_pending,
            self.stats_failed, self.total_executed);

        for q_name in ["P(높음)", "O(보통)", "T(낮음)"] {
            let q = match q_name {
                "P(높음)" => &self.queue_high,
                "O(보통)" => &self.queue_normal,
                _ => &self.queue_low,
            };
            if !q.is_empty() {
                println!("║ ── {} 큐 ──", q_name);
                for t in q {
                    println!("║   [{:04}] {} ({})", t.id, t.name, t.state);
                }
            }
        }
        println!("╚═══════════════════════════════════════════╝");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_basic() {
        let mut sched = TritScheduler::new();

        sched.submit("작업A", TritPriority::High, Box::new(|| TritResult::Success));
        sched.submit("작업B", TritPriority::Normal, Box::new(|| TritResult::Success));
        sched.submit("작업C", TritPriority::Low, Box::new(|| TritResult::Success));

        let results = sched.run_all();
        assert_eq!(results.len(), 3);
        // 우선순위 순: A(P) → B(O) → C(T)
        assert_eq!(results[0].1, TritResult::Success);
    }

    #[test]
    fn test_scheduler_retry() {
        let mut sched = TritScheduler::new();

        // Pending 반환하면 재시도
        sched.submit("재시도작업", TritPriority::High, Box::new(|| TritResult::Pending));

        let r = sched.execute_one().unwrap();
        assert_eq!(r.1, TritResult::Pending);
        // 큐에 다시 들어갔는지 확인
        assert_eq!(sched.pending_count(), 1);
    }

    #[test]
    fn test_scheduler_cancel() {
        let mut sched = TritScheduler::new();
        let id = sched.submit("취소될작업", TritPriority::Normal, Box::new(|| TritResult::Success));

        assert!(sched.cancel(id));
        let r = sched.execute_one().unwrap();
        assert_eq!(r.1, TritResult::Failed);
    }
}
