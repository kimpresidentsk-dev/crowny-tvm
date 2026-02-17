///! ═══════════════════════════════════════════════════
///! CAR — Crowny Application Runtime v0.1
///! ═══════════════════════════════════════════════════
///!
///! Meta-Kernel ↔ 애플리케이션 사이의 표준 실행 계층.
///!
///! 모든 앱(한선어, 웹서버, LLM 등)은 직접 커널 호출 금지.
///! 반드시 CAR.submit()을 통해 Task로 제출.
///!
///! GPT Spec: "직접 Meta-Kernel 호출 금지,
///!            모든 실행은 CAR.submit() 경유"
///!
///! 실행 흐름:
///!   앱 → CAR.submit(AppTask) → 권한검사 → 스케줄 → TVM 실행 → TritResult

use std::collections::HashMap;
use std::time::Instant;

// ─────────────────────────────────────────────
// TritResult — 표준 반환 타입
// ─────────────────────────────────────────────

/// 3진 상태 (모든 반환에 사용)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum TritState {
    Success =  1,  // P: 성공
    Pending =  0,  // O: 보류/진행중
    Failed  = -1,  // T: 실패
}

impl TritState {
    pub fn from_i8(v: i8) -> Self {
        match v {
            1 => TritState::Success,
            -1 => TritState::Failed,
            _ => TritState::Pending,
        }
    }

    pub fn symbol(self) -> char {
        match self {
            TritState::Success => 'P',
            TritState::Pending => 'O',
            TritState::Failed => 'T',
        }
    }
}

impl std::fmt::Display for TritState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TritState::Success => write!(f, "P(성공)"),
            TritState::Pending => write!(f, "O(보류)"),
            TritState::Failed => write!(f, "T(실패)"),
        }
    }
}

/// 표준 반환 — 모든 CAR 작업의 결과
#[derive(Debug, Clone)]
pub struct TritResult {
    pub state: TritState,
    pub data: ResultData,
    pub elapsed_ms: u64,
    pub task_id: u64,
}

/// 반환 데이터 종류
#[derive(Debug, Clone)]
pub enum ResultData {
    None,
    Integer(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
    Trit(i8),
    List(Vec<ResultData>),
    Map(HashMap<String, ResultData>),
}

impl std::fmt::Display for ResultData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResultData::None => write!(f, "없음"),
            ResultData::Integer(n) => write!(f, "{}", n),
            ResultData::Float(v) => write!(f, "{}", v),
            ResultData::Text(s) => write!(f, "{}", s),
            ResultData::Bytes(b) => write!(f, "[{} bytes]", b.len()),
            ResultData::Trit(t) => write!(f, "Trit({})", t),
            ResultData::List(l) => write!(f, "[{} items]", l.len()),
            ResultData::Map(m) => write!(f, "{{{} entries}}", m.len()),
        }
    }
}

// ─────────────────────────────────────────────
// AppTask — 애플리케이션 작업 정의
// ─────────────────────────────────────────────

/// 작업 유형
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    Compile,    // 한선어 컴파일
    Execute,    // TVM 프로그램 실행
    WebRequest, // 웹 요청 처리
    LlmCall,    // LLM API 호출
    DbQuery,    // DB 조회
    FileIO,     // 파일 입출력
    System,     // 시스템 명령
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskType::Compile => write!(f, "컴파일"),
            TaskType::Execute => write!(f, "실행"),
            TaskType::WebRequest => write!(f, "웹요청"),
            TaskType::LlmCall => write!(f, "LLM"),
            TaskType::DbQuery => write!(f, "DB"),
            TaskType::FileIO => write!(f, "파일"),
            TaskType::System => write!(f, "시스템"),
        }
    }
}

/// 앱 작업 요청
#[derive(Debug, Clone)]
pub struct AppTask {
    pub task_type: TaskType,
    pub subject: String,     // 요청자
    pub payload: String,     // 페이로드 (소스코드, URL, 프롬프트 등)
    pub params: HashMap<String, String>,  // 추가 파라미터
}

impl AppTask {
    pub fn new(task_type: TaskType, subject: &str, payload: &str) -> Self {
        Self {
            task_type,
            subject: subject.to_string(),
            payload: payload.to_string(),
            params: HashMap::new(),
        }
    }

    pub fn with_param(mut self, key: &str, val: &str) -> Self {
        self.params.insert(key.to_string(), val.to_string());
        self
    }
}

// ─────────────────────────────────────────────
// CAR — Crowny Application Runtime
// ─────────────────────────────────────────────

/// 권한 레벨 (간소화)
#[derive(Debug, Clone, Copy)]
pub enum AccessLevel {
    Public,     // 누구나
    User,       // 인증 사용자
    Admin,      // 관리자
    Kernel,     // 커널 전용
}

/// 작업 이력
#[derive(Debug)]
struct TaskLog {
    task_id: u64,
    task_type: TaskType,
    subject: String,
    state: TritState,
    elapsed_ms: u64,
}

/// Crowny Application Runtime
pub struct CrownyRuntime {
    task_counter: u64,
    history: Vec<TaskLog>,
    // 권한 매핑: TaskType → 최소 AccessLevel
    access_rules: HashMap<String, AccessLevel>,
    // 통계
    success_count: u64,
    pending_count: u64,
    failed_count: u64,
}

impl CrownyRuntime {
    pub fn new() -> Self {
        let mut access_rules = HashMap::new();
        // 기본 접근 정책
        access_rules.insert("Compile".into(), AccessLevel::User);
        access_rules.insert("Execute".into(), AccessLevel::User);
        access_rules.insert("WebRequest".into(), AccessLevel::Public);
        access_rules.insert("LlmCall".into(), AccessLevel::User);
        access_rules.insert("DbQuery".into(), AccessLevel::User);
        access_rules.insert("FileIO".into(), AccessLevel::Admin);
        access_rules.insert("System".into(), AccessLevel::Kernel);

        println!("[CAR] Crowny Application Runtime 시작");
        Self {
            task_counter: 0,
            history: Vec::new(),
            access_rules,
            success_count: 0,
            pending_count: 0,
            failed_count: 0,
        }
    }

    /// 핵심 메서드: 작업 제출
    /// 모든 앱은 이것만 호출한다.
    pub fn submit(
        &mut self,
        task: AppTask,
        executor: impl FnOnce(&AppTask) -> (TritState, ResultData),
    ) -> TritResult {
        let start = Instant::now();
        self.task_counter += 1;
        let task_id = self.task_counter;

        // 1. 권한 검사
        let access_ok = self.check_access(&task);
        if !access_ok {
            self.failed_count += 1;
            self.log_task(task_id, &task, TritState::Failed, 0);
            return TritResult {
                state: TritState::Failed,
                data: ResultData::Text("권한 부족".into()),
                elapsed_ms: 0,
                task_id,
            };
        }

        // 2. 실행
        let (state, data) = executor(&task);

        let elapsed = start.elapsed().as_millis() as u64;

        // 3. 통계 업데이트
        match state {
            TritState::Success => self.success_count += 1,
            TritState::Pending => self.pending_count += 1,
            TritState::Failed => self.failed_count += 1,
        }

        // 4. 이력 기록
        self.log_task(task_id, &task, state, elapsed);

        // 5. 표준 결과 반환
        TritResult { state, data, elapsed_ms: elapsed, task_id }
    }

    /// 간편 실행: 소스코드 컴파일+실행
    pub fn run_source(&mut self, subject: &str, source: &str) -> TritResult {
        let task = AppTask::new(TaskType::Execute, subject, source);
        self.submit(task, |t| {
            // 어셈블 + TVM 실행
            let program = crate::assembler::assemble(&t.payload);
            if program.is_empty() {
                return (TritState::Failed, ResultData::Text("빈 프로그램".into()));
            }
            let mut vm = crate::vm::TVM::new();
            vm.load(program);
            match vm.run() {
                Ok(()) => {
                    let top = vm.stack.last()
                        .and_then(|v| v.as_int())
                        .unwrap_or(0);
                    (TritState::Success, ResultData::Integer(top))
                }
                Err(e) => (TritState::Failed, ResultData::Text(format!("{:?}", e))),
            }
        })
    }

    /// 간편 실행: WASM 컴파일
    pub fn compile_wasm(&mut self, subject: &str, source: &str) -> TritResult {
        let task = AppTask::new(TaskType::Compile, subject, source);
        self.submit(task, |t| {
            let result = crate::compiler::compile_with_info(&t.payload, "crowny");
            if result.wasm_bytes.is_empty() {
                (TritState::Failed, ResultData::Text("컴파일 실패".into()))
            } else {
                (TritState::Success, ResultData::Bytes(result.wasm_bytes))
            }
        })
    }

    fn check_access(&self, _task: &AppTask) -> bool {
        // 간소화: 현재 모든 접근 허용 (v0.1)
        // 실제로는 Permission Engine과 연동
        true
    }

    fn log_task(&mut self, task_id: u64, task: &AppTask, state: TritState, elapsed_ms: u64) {
        self.history.push(TaskLog {
            task_id,
            task_type: task.task_type,
            subject: task.subject.clone(),
            state,
            elapsed_ms,
        });
    }

    /// 상태 출력
    pub fn dump(&self) {
        println!("╔══ CAR 상태 ════════════════════════════╗");
        println!("║ 총 작업: {} | P:{} O:{} T:{}",
            self.task_counter, self.success_count, self.pending_count, self.failed_count);
        let recent = self.history.iter().rev().take(5);
        for log in recent {
            println!("║  [{}] {}:{} → {} ({}ms)",
                log.task_id, log.subject, log.task_type, log.state, log.elapsed_ms);
        }
        println!("╚═══════════════════════════════════════╝");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_car_execute() {
        let mut car = CrownyRuntime::new();
        let result = car.run_source("테스트", "넣어 5\n넣어 3\n더해\n종료");
        assert_eq!(result.state, TritState::Success);
        if let ResultData::Integer(v) = result.data {
            assert_eq!(v, 8);
        }
    }

    #[test]
    fn test_car_compile_wasm() {
        let mut car = CrownyRuntime::new();
        let result = car.compile_wasm("테스트", "넣어 42\n종료");
        assert_eq!(result.state, TritState::Success);
        if let ResultData::Bytes(wasm) = &result.data {
            assert_eq!(&wasm[0..4], b"\0asm");
        }
    }
}
