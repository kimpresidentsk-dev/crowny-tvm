///! ═══════════════════════════════════════════════════
///! Trit Debugger v0.1
///! ═══════════════════════════════════════════════════
///!
///! 3진 시스템은 2진보다 디버깅이 복잡하다.
///! 상태가 3가지이므로 전이 추적이 핵심.
///!
///! 기능:
///!   - 단계 실행 (Step)
///!   - 브레이크포인트
///!   - 스택 덤프
///!   - 힙 덤프
///!   - Trit 상태 전이 추적
///!   - 명령어 흐름 트레이스
///!   - 권한 충돌 표시
///!   - 실행 프로파일링

use std::collections::HashMap;
use crate::vm::{TVM, Instruction, VmError};
use crate::opcode::{OpcodeAddr, build_opcodes, OpMeta};
use crate::value::Value;

// ─────────────────────────────────────────────
// 디버그 이벤트
// ─────────────────────────────────────────────

/// 디버그 이벤트 종류
#[derive(Debug, Clone)]
pub enum DebugEvent {
    /// 명령어 실행
    Execute {
        pc: usize,
        addr: OpcodeAddr,
        name: String,
        stack_before: Vec<String>,
        stack_after: Vec<String>,
    },
    /// 브레이크포인트 도달
    Breakpoint { pc: usize, reason: String },
    /// 상태 전이
    StateChange { from: i8, to: i8, context: String },
    /// 스택 변화
    StackChange { op: String, popped: Vec<String>, pushed: Vec<String> },
    /// 오류
    Error { pc: usize, message: String },
    /// 프로그램 종료
    Halt { pc: usize, final_stack: Vec<String> },
}

// ─────────────────────────────────────────────
// 디버거
// ─────────────────────────────────────────────

/// Trit 디버거
pub struct TritDebugger {
    vm: TVM,
    program: Vec<Instruction>,
    opcodes: HashMap<OpcodeAddr, OpMeta>,
    // 브레이크포인트
    breakpoints: Vec<usize>,
    // 실행 트레이스
    trace: Vec<DebugEvent>,
    // 실행 통계
    exec_count: HashMap<OpcodeAddr, usize>,
    step_count: usize,
    max_steps: usize,
    // 설정
    trace_enabled: bool,
}

impl TritDebugger {
    pub fn new(program: Vec<Instruction>) -> Self {
        let opcodes = build_opcodes();
        Self {
            vm: TVM::new(),
            program: program.clone(),
            opcodes,
            breakpoints: Vec::new(),
            trace: Vec::new(),
            exec_count: HashMap::new(),
            step_count: 0,
            max_steps: 10000,
            trace_enabled: true,
        }
    }

    /// 소스에서 디버거 생성
    pub fn from_source(source: &str) -> Self {
        let program = crate::assembler::assemble(source);
        Self::new(program)
    }

    /// 브레이크포인트 설정
    pub fn set_breakpoint(&mut self, pc: usize) {
        if !self.breakpoints.contains(&pc) {
            self.breakpoints.push(pc);
        }
    }

    /// 브레이크포인트 해제
    pub fn clear_breakpoint(&mut self, pc: usize) {
        self.breakpoints.retain(|&bp| bp != pc);
    }

    /// 최대 실행 스텝 설정
    pub fn set_max_steps(&mut self, max: usize) {
        self.max_steps = max;
    }

    /// 프로그램 로드
    pub fn load(&mut self) {
        self.vm.load(self.program.clone());
        self.trace.clear();
        self.exec_count.clear();
        self.step_count = 0;
    }

    /// 단계 실행 (1스텝)
    pub fn step(&mut self) -> Result<DebugEvent, VmError> {
        self.step_count += 1;
        if self.step_count > self.max_steps {
            return Err(VmError::Custom("최대 스텝 초과".into()));
        }

        let ip = self.vm.ip;

        // 프로그램 범위 초과
        if ip >= self.vm.program.len() {
            let event = DebugEvent::Halt {
                pc: ip,
                final_stack: self.stack_snapshot(),
            };
            if self.trace_enabled { self.trace.push(event.clone()); }
            return Ok(event);
        }

        let inst = &self.vm.program[ip];
        let addr = inst.addr;
        let name = self.opcodes.get(&addr).map(|m| m.name_kr).unwrap_or("???").to_string();

        // 실행 전 스택
        let stack_before = self.stack_snapshot();

        // 브레이크포인트 체크 (실행 후 BP 이벤트 반환)
        let hit_bp = self.breakpoints.contains(&ip);
        if hit_bp {
            // BP는 1회만 발동 (무한루프 방지)
            self.breakpoints.retain(|&b| b != ip);
        }

        // 실행
        match self.vm.step() {
            Ok(continue_run) => {
                let stack_after = self.stack_snapshot();
                *self.exec_count.entry(addr).or_insert(0) += 1;

                if hit_bp {
                    let event = DebugEvent::Breakpoint {
                        pc: ip,
                        reason: format!("BP@{}: {} {}", ip, name, addr),
                    };
                    if self.trace_enabled { self.trace.push(event.clone()); }
                    return Ok(event);
                }

                let event = DebugEvent::Execute {
                    pc: ip,
                    addr,
                    name,
                    stack_before,
                    stack_after,
                };
                if self.trace_enabled { self.trace.push(event.clone()); }

                if !continue_run {
                    let halt = DebugEvent::Halt {
                        pc: self.vm.ip,
                        final_stack: self.stack_snapshot(),
                    };
                    if self.trace_enabled { self.trace.push(halt); }
                }

                Ok(event)
            }
            Err(e) => {
                let event = DebugEvent::Error {
                    pc: ip,
                    message: format!("{:?}", e),
                };
                if self.trace_enabled { self.trace.push(event.clone()); }
                Err(e)
            }
        }
    }

    /// 브레이크포인트까지 실행
    pub fn run_to_breakpoint(&mut self) -> Vec<DebugEvent> {
        let mut events = Vec::new();
        loop {
            match self.step() {
                Ok(event) => {
                    let is_bp = matches!(&event, DebugEvent::Breakpoint { .. });
                    let is_halt = matches!(&event, DebugEvent::Halt { .. });
                    events.push(event);
                    if is_bp || is_halt { break; }
                }
                Err(_) => break,
            }
            if self.step_count > self.max_steps { break; }
        }
        events
    }

    /// 전체 실행 (트레이스 수집)
    pub fn run_all(&mut self) -> Vec<DebugEvent> {
        self.load();
        let mut events = Vec::new();
        loop {
            match self.step() {
                Ok(event) => {
                    let is_halt = matches!(&event, DebugEvent::Halt { .. });
                    events.push(event);
                    if is_halt { break; }
                }
                Err(_) => break,
            }
            if self.step_count > self.max_steps { break; }
        }
        events
    }

    // ── 스냅샷 ──

    fn stack_snapshot(&self) -> Vec<String> {
        self.vm.stack.iter().map(|v| format!("{}", v)).collect()
    }

    /// 현재 스택 덤프
    pub fn dump_stack(&self) -> String {
        let mut out = String::new();
        out.push_str("┌── 스택 ──────────────────────┐\n");
        if self.vm.stack.is_empty() {
            out.push_str("│ (비어있음)                    │\n");
        } else {
            for (i, val) in self.vm.stack.iter().enumerate().rev() {
                let marker = if i == self.vm.stack.len() - 1 { "→" } else { " " };
                out.push_str(&format!("│ {} [{}] {}\n", marker, i, val));
            }
        }
        out.push_str("└──────────────────────────────┘\n");
        out
    }

    /// 프로그램 디스어셈블리 (현재 PC 표시)
    pub fn dump_program(&self) -> String {
        let mut out = String::new();
        out.push_str("┌── 프로그램 ───────────────────┐\n");
        for (i, inst) in self.program.iter().enumerate() {
            let marker = if i == self.vm.ip { "→" } else { " " };
            let bp = if self.breakpoints.contains(&i) { "●" } else { " " };
            let name = self.opcodes.get(&inst.addr).map(|m| m.name_kr).unwrap_or("???");
            let ops = if inst.operands.is_empty() {
                String::new()
            } else {
                format!(" {}", inst.operands.iter().map(|v| format!("{}", v)).collect::<Vec<_>>().join(","))
            };
            out.push_str(&format!("│ {}{} {:04}: {} {}{}\n", marker, bp, i, inst.addr, name, ops));
        }
        out.push_str("└──────────────────────────────┘\n");
        out
    }

    /// 실행 프로파일
    pub fn profile(&self) -> String {
        let mut out = String::new();
        out.push_str("┌── 프로파일 ───────────────────┐\n");
        out.push_str(&format!("│ 총 스텝: {}\n", self.step_count));

        let mut sorted: Vec<_> = self.exec_count.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));

        for (addr, count) in sorted.iter().take(10) {
            let name = self.opcodes.get(addr).map(|m| m.name_kr).unwrap_or("???");
            let pct = (*count * 100) as f32 / self.step_count.max(1) as f32;
            out.push_str(&format!("│ {:8} {} {:.1}%\n", name, count, pct));
        }
        out.push_str("└──────────────────────────────┘\n");
        out
    }

    /// 트레이스 출력
    pub fn dump_trace(&self) -> String {
        let mut out = String::new();
        out.push_str("┌── 실행 트레이스 ─────────────┐\n");
        for (i, event) in self.trace.iter().enumerate() {
            match event {
                DebugEvent::Execute { pc, name, stack_before, stack_after, .. } => {
                    let before_len = stack_before.len();
                    let after_len = stack_after.len();
                    let top = stack_after.last().map(|s| s.as_str()).unwrap_or("-");
                    out.push_str(&format!("│ {:04} [{}] {} — 스택:{}/{} top:{}\n",
                        i, pc, name, before_len, after_len, top));
                }
                DebugEvent::Breakpoint { pc, reason } => {
                    out.push_str(&format!("│ {:04} ● BP@{}: {}\n", i, pc, reason));
                }
                DebugEvent::Halt { pc, final_stack } => {
                    let top = final_stack.last().map(|s| s.as_str()).unwrap_or("-");
                    out.push_str(&format!("│ {:04} ■ HALT@{} — 최종값:{}\n", i, pc, top));
                }
                DebugEvent::Error { pc, message } => {
                    out.push_str(&format!("│ {:04} ✗ ERR@{}: {}\n", i, pc, message));
                }
                _ => {}
            }
        }
        out.push_str("└──────────────────────────────┘\n");
        out
    }

    /// 최종 결과
    pub fn result_value(&self) -> Option<i64> {
        self.vm.stack.last().and_then(|v| v.as_int())
    }

    /// 트레이스 길이
    pub fn trace_len(&self) -> usize {
        self.trace.len()
    }
}

// ─────────────────────────────────────────────
// 대화형 디버거 (CLI)
// ─────────────────────────────────────────────

/// 디버그 명령
pub enum DebugCmd {
    Step,               // s: 한 단계
    Run,                // r: 끝까지 실행
    RunToBp,            // c: 브레이크포인트까지
    Stack,              // stack: 스택 덤프
    Program,            // prog: 프로그램 보기
    Trace,              // trace: 트레이스
    Profile,            // prof: 프로파일
    Break(usize),       // b N: 브레이크포인트 설정
    ClearBreak(usize),  // cb N: 해제
    Info,               // info: 상태 정보
    Quit,               // q: 종료
    Help,               // h: 도움말
    Unknown,
}

/// 명령 파싱
pub fn parse_debug_cmd(input: &str) -> DebugCmd {
    let input = input.trim();
    let parts: Vec<&str> = input.split_whitespace().collect();
    match parts.first().map(|s| *s) {
        Some("s") | Some("step") | Some("단계") => DebugCmd::Step,
        Some("r") | Some("run") | Some("실행") => DebugCmd::Run,
        Some("c") | Some("continue") | Some("계속") => DebugCmd::RunToBp,
        Some("stack") | Some("스택") => DebugCmd::Stack,
        Some("prog") | Some("program") | Some("프로그램") => DebugCmd::Program,
        Some("trace") | Some("트레이스") => DebugCmd::Trace,
        Some("prof") | Some("profile") | Some("프로파일") => DebugCmd::Profile,
        Some("b") | Some("break") | Some("중단점") => {
            let n = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            DebugCmd::Break(n)
        }
        Some("cb") | Some("clear") | Some("해제") => {
            let n = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            DebugCmd::ClearBreak(n)
        }
        Some("info") | Some("정보") => DebugCmd::Info,
        Some("q") | Some("quit") | Some("종료") => DebugCmd::Quit,
        Some("h") | Some("help") | Some("도움") => DebugCmd::Help,
        _ => DebugCmd::Unknown,
    }
}

/// 디버거 도움말
pub fn debug_help() -> &'static str {
    concat!(
        "┌── Trit Debugger 명령어 ───────┐\n",
        "│ s/step/단계     1스텝 실행      │\n",
        "│ r/run/실행      전체 실행        │\n",
        "│ c/continue/계속 BP까지 실행      │\n",
        "│ stack/스택      스택 덤프         │\n",
        "│ prog/프로그램   프로그램 보기      │\n",
        "│ trace/트레이스  실행 트레이스      │\n",
        "│ prof/프로파일   실행 통계         │\n",
        "│ b N/중단점 N    브레이크포인트     │\n",
        "│ cb N/해제 N     BP 해제          │\n",
        "│ info/정보       상태 정보         │\n",
        "│ q/quit/종료     디버거 종료       │\n",
        "│ h/help/도움     이 도움말         │\n",
        "└──────────────────────────────┘\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debugger_basic() {
        let mut dbg = TritDebugger::from_source("넣어 5\n넣어 3\n더해\n종료");
        dbg.load();
        let events = dbg.run_all();
        assert!(events.len() >= 4);
        assert_eq!(dbg.result_value(), Some(8));
    }

    #[test]
    fn test_step_execution() {
        let mut dbg = TritDebugger::from_source("넣어 10\n넣어 20\n더해\n종료");
        dbg.load();

        // 스텝 1: 넣어 10
        let event = dbg.step().unwrap();
        assert!(matches!(event, DebugEvent::Execute { name, .. } if name == "넣어"));

        // 스텝 2: 넣어 20
        let event = dbg.step().unwrap();
        assert!(matches!(event, DebugEvent::Execute { name, .. } if name == "넣어"));

        // 스텝 3: 더해
        let event = dbg.step().unwrap();
        assert!(matches!(event, DebugEvent::Execute { name, .. } if name == "더해"));

        // 결과 확인
        assert_eq!(dbg.result_value(), Some(30));
    }

    #[test]
    fn test_breakpoint() {
        let mut dbg = TritDebugger::from_source("넣어 1\n넣어 2\n넣어 3\n더해\n종료");
        dbg.load();
        dbg.set_breakpoint(2); // PC=2에 BP

        let events = dbg.run_to_breakpoint();
        // BP에서 멈춤
        let has_bp = events.iter().any(|e| matches!(e, DebugEvent::Breakpoint { .. }));
        assert!(has_bp);
    }

    #[test]
    fn test_profile() {
        let mut dbg = TritDebugger::from_source("넣어 1\n넣어 2\n넣어 3\n더해\n더해\n종료");
        dbg.run_all();
        let profile = dbg.profile();
        assert!(profile.contains("넣어"));
        assert!(profile.contains("더해"));
    }

    #[test]
    fn test_trace() {
        let mut dbg = TritDebugger::from_source("넣어 42\n종료");
        dbg.run_all();
        assert!(dbg.trace_len() >= 2);
        let trace = dbg.dump_trace();
        assert!(trace.contains("넣어"));
    }
}
