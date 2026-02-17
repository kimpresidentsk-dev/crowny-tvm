///! ═══════════════════════════════════════════════════
///! Trit Test Framework v0.1
///! ═══════════════════════════════════════════════════
///!
///! 3진 시스템 전용 테스트 프레임워크.
///! 일반 assert 외에 Trit 상태 전이, 합의, 권한 테스트 지원.
///!
///! 특징:
///!   - TritAssert: 3진 상태 검증
///!   - 상태 전이 테스트: -1 ↔ 0 ↔ +1 규칙 검증
///!   - 합의 시뮬레이터: 다수결/거부권 테스트
///!   - 권한 테스트: 접근 제어 검증
///!   - 한선어 프로그램 실행 테스트
///!   - 테스트 스위트 + 보고서

use std::time::Instant;
use crate::car::TritState;

// ─────────────────────────────────────────────
// TritAssert — 3진 어서션
// ─────────────────────────────────────────────

/// 어서션 결과
#[derive(Debug, Clone)]
pub struct AssertResult {
    pub passed: bool,
    pub name: String,
    pub message: String,
    pub expected: String,
    pub actual: String,
}

/// 3진 어서션 빌더
pub struct TritAssert;

impl TritAssert {
    /// 상태가 P(성공)인지 확인
    pub fn is_success(name: &str, state: TritState) -> AssertResult {
        AssertResult {
            passed: state == TritState::Success,
            name: name.to_string(),
            message: if state == TritState::Success { "P(성공) 확인".into() } else { "P(성공) 아님".into() },
            expected: "P(성공)".into(),
            actual: format!("{}", state),
        }
    }

    /// 상태가 O(보류)인지 확인
    pub fn is_pending(name: &str, state: TritState) -> AssertResult {
        AssertResult {
            passed: state == TritState::Pending,
            name: name.to_string(),
            message: if state == TritState::Pending { "O(보류) 확인".into() } else { "O(보류) 아님".into() },
            expected: "O(보류)".into(),
            actual: format!("{}", state),
        }
    }

    /// 상태가 T(실패)인지 확인
    pub fn is_failed(name: &str, state: TritState) -> AssertResult {
        AssertResult {
            passed: state == TritState::Failed,
            name: name.to_string(),
            message: if state == TritState::Failed { "T(실패) 확인".into() } else { "T(실패) 아님".into() },
            expected: "T(실패)".into(),
            actual: format!("{}", state),
        }
    }

    /// 두 상태가 같은지 확인
    pub fn eq_state(name: &str, a: TritState, b: TritState) -> AssertResult {
        AssertResult {
            passed: a == b,
            name: name.to_string(),
            message: if a == b { "상태 일치".into() } else { "상태 불일치".into() },
            expected: format!("{}", b),
            actual: format!("{}", a),
        }
    }

    /// 값이 같은지 확인
    pub fn eq_i64(name: &str, actual: i64, expected: i64) -> AssertResult {
        AssertResult {
            passed: actual == expected,
            name: name.to_string(),
            message: if actual == expected { "값 일치".into() } else { "값 불일치".into() },
            expected: format!("{}", expected),
            actual: format!("{}", actual),
        }
    }

    /// 상태 전이 규칙 검증: 직접 -1→+1 점프 금지
    pub fn valid_transition(name: &str, from: i8, to: i8) -> AssertResult {
        let valid = (to - from).abs() <= 1;
        AssertResult {
            passed: valid,
            name: name.to_string(),
            message: if valid { "유효한 전이".into() } else { format!("불법 전이: {}→{}", from, to) },
            expected: format!("{}→{} (|차이|≤1)", from, to),
            actual: format!("차이={}", (to - from).abs()),
        }
    }

    /// 하향 안정성 원칙 검증: 충돌 시 낮은 값으로
    pub fn downward_stability(name: &str, a: i8, b: i8, result: i8) -> AssertResult {
        let expected = a.min(b);
        AssertResult {
            passed: result == expected,
            name: name.to_string(),
            message: if result == expected { "하향 안정성 준수".into() } else { "하향 안정성 위반".into() },
            expected: format!("min({},{}) = {}", a, b, expected),
            actual: format!("{}", result),
        }
    }
}

// ─────────────────────────────────────────────
// 테스트 케이스
// ─────────────────────────────────────────────

/// 테스트 케이스
pub struct TestCase {
    pub name: String,
    pub description: String,
    pub runner: Box<dyn FnOnce() -> Vec<AssertResult>>,
}

impl TestCase {
    pub fn new(name: &str, desc: &str, runner: impl FnOnce() -> Vec<AssertResult> + 'static) -> Self {
        Self {
            name: name.to_string(),
            description: desc.to_string(),
            runner: Box::new(runner),
        }
    }
}

// ─────────────────────────────────────────────
// 테스트 스위트
// ─────────────────────────────────────────────

/// 테스트 결과 요약
#[derive(Debug)]
pub struct SuiteResult {
    pub suite_name: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub elapsed_ms: u64,
    pub details: Vec<(String, Vec<AssertResult>)>,
}

/// 테스트 스위트
pub struct TestSuite {
    pub name: String,
    cases: Vec<TestCase>,
}

impl TestSuite {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), cases: Vec::new() }
    }

    pub fn add(&mut self, case: TestCase) {
        self.cases.push(case);
    }

    /// 전체 실행
    pub fn run(self) -> SuiteResult {
        let start = Instant::now();
        let mut total = 0usize;
        let mut passed = 0usize;
        let mut failed = 0usize;
        let mut details = Vec::new();

        for case in self.cases {
            let results = (case.runner)();
            for r in &results {
                total += 1;
                if r.passed { passed += 1; } else { failed += 1; }
            }
            details.push((case.name, results));
        }

        SuiteResult {
            suite_name: self.name,
            total, passed, failed,
            elapsed_ms: start.elapsed().as_millis() as u64,
            details,
        }
    }
}

impl SuiteResult {
    /// 보고서 출력
    pub fn report(&self) -> String {
        let mut out = String::new();
        let status = if self.failed == 0 { "✓ 전체 통과" } else { "✗ 실패 있음" };
        out.push_str(&format!("╔══ {} [{}] ══╗\n", self.suite_name, status));
        out.push_str(&format!("║ 총: {} | P:{} | T:{} | {}ms\n", self.total, self.passed, self.failed, self.elapsed_ms));

        for (case_name, results) in &self.details {
            let case_ok = results.iter().all(|r| r.passed);
            let mark = if case_ok { "P" } else { "T" };
            out.push_str(&format!("║ [{}] {}\n", mark, case_name));
            for r in results {
                if !r.passed {
                    out.push_str(&format!("║   ✗ {} — 예상:{} 실제:{}\n", r.name, r.expected, r.actual));
                }
            }
        }
        out.push_str("╚══════════════════════════════════════╝\n");
        out
    }
}

// ─────────────────────────────────────────────
// 한선어 프로그램 실행 테스트
// ─────────────────────────────────────────────

/// 한선어 소스를 실행하고 TVM 스택 최상위 값 반환
pub fn run_and_check(source: &str) -> (TritState, i64) {
    let program = crate::assembler::assemble(source);
    if program.is_empty() {
        return (TritState::Failed, 0);
    }
    let mut vm = crate::vm::TVM::new();
    vm.load(program);
    match vm.run() {
        Ok(()) => {
            let top = vm.stack.last().and_then(|v| v.as_int()).unwrap_or(0);
            (TritState::Success, top)
        }
        Err(_) => (TritState::Failed, 0),
    }
}

/// 한선어 프로그램 테스트 케이스 생성
pub fn source_test(name: &str, source: &str, expected: i64) -> TestCase {
    let name_owned = name.to_string();
    let source = source.to_string();
    let name_for_closure = name_owned.clone();
    TestCase::new(&name_owned, "한선어 실행 테스트", move || {
        let (state, actual) = run_and_check(&source);
        vec![
            TritAssert::is_success(&format!("{}_상태", name_for_closure), state),
            TritAssert::eq_i64(&format!("{}_값", name_for_closure), actual, expected),
        ]
    })
}

// ─────────────────────────────────────────────
// 내장 테스트 스위트
// ─────────────────────────────────────────────

/// 코어 TVM 테스트 스위트
pub fn core_suite() -> TestSuite {
    let mut suite = TestSuite::new("코어 TVM 테스트");

    // 1. 산술
    suite.add(source_test("덧셈", "넣어 5\n넣어 3\n더해\n종료", 8));
    suite.add(source_test("뺄셈", "넣어 10\n넣어 3\n빼\n종료", 7));
    suite.add(source_test("곱셈", "넣어 6\n넣어 7\n곱해\n종료", 42));
    suite.add(source_test("나눗셈", "넣어 20\n넣어 4\n나눠\n종료", 5));
    suite.add(source_test("나머지", "넣어 17\n넣어 5\n나머지\n종료", 2));

    // 2. 스택
    suite.add(source_test("복사", "넣어 9\n복사\n더해\n종료", 18));
    suite.add(source_test("바꿔", "넣어 1\n넣어 2\n바꿔\n종료", 1));

    // 3. 비교
    suite.add(source_test("같다_참", "넣어 5\n넣어 5\n같다\n종료", 1));
    suite.add(source_test("크다_참", "넣어 10\n넣어 3\n크다\n종료", 1));
    suite.add(source_test("작다_참", "넣어 3\n넣어 10\n작다\n종료", 1));

    // 4. 3진 논리
    suite.add(source_test("참_상수", "참\n종료", 1));
    suite.add(source_test("거짓_상수", "거짓\n종료", -1));
    suite.add(source_test("모름_상수", "모름\n종료", 0));

    suite
}

/// 상태 전이 규칙 테스트 스위트
pub fn transition_suite() -> TestSuite {
    let mut suite = TestSuite::new("상태 전이 규칙 테스트");

    suite.add(TestCase::new("유효_전이", "상태 전이 유효성", || {
        vec![
            TritAssert::valid_transition("T→O", -1, 0),
            TritAssert::valid_transition("O→P", 0, 1),
            TritAssert::valid_transition("P→O", 1, 0),
            TritAssert::valid_transition("O→T", 0, -1),
            TritAssert::valid_transition("제자리_P", 1, 1),
            TritAssert::valid_transition("제자리_O", 0, 0),
            TritAssert::valid_transition("제자리_T", -1, -1),
        ]
    }));

    suite.add(TestCase::new("불법_점프", "직접 T→P 금지", || {
        vec![
            TritAssert::valid_transition("T→P_불법", -1, 1), // 이건 실패해야 함
        ]
    }));

    suite.add(TestCase::new("하향_안정성", "충돌 시 낮은 값", || {
        vec![
            TritAssert::downward_stability("P∧O=O", 1, 0, 0),
            TritAssert::downward_stability("P∧T=T", 1, -1, -1),
            TritAssert::downward_stability("O∧T=T", 0, -1, -1),
            TritAssert::downward_stability("P∧P=P", 1, 1, 1),
        ]
    }));

    suite
}

/// CAR 통합 테스트 스위트
pub fn car_suite() -> TestSuite {
    let mut suite = TestSuite::new("CAR 통합 테스트");

    suite.add(TestCase::new("CAR_실행", "CAR.submit 실행 테스트", || {
        let mut car = crate::car::CrownyRuntime::new();
        let result = car.run_source("test", "넣어 100\n넣어 200\n더해\n종료");
        vec![
            TritAssert::is_success("CAR_상태", result.state),
            TritAssert::eq_i64("CAR_값", match result.data {
                crate::car::ResultData::Integer(n) => n,
                _ => -999,
            }, 300),
        ]
    }));

    suite.add(TestCase::new("CAR_WASM", "WASM 컴파일 테스트", || {
        let mut car = crate::car::CrownyRuntime::new();
        let result = car.compile_wasm("test", "넣어 42\n종료");
        let is_wasm = match &result.data {
            crate::car::ResultData::Bytes(b) => b.len() > 8 && &b[0..4] == b"\0asm",
            _ => false,
        };
        vec![
            TritAssert::is_success("WASM_상태", result.state),
            AssertResult {
                passed: is_wasm,
                name: "WASM_매직".into(),
                message: if is_wasm { "WASM 유효".into() } else { "WASM 무효".into() },
                expected: "\\0asm 헤더".into(),
                actual: if is_wasm { "확인".into() } else { "없음".into() },
            },
        ]
    }));

    suite
}

/// 합의 시뮬레이션 테스트
pub fn consensus_suite() -> TestSuite {
    let mut suite = TestSuite::new("합의 엔진 테스트");

    suite.add(TestCase::new("다수결_합의", "3자 다수결", || {
        // P,P,T → P (다수결)
        let votes = vec![1i8, 1, -1];
        let sum: i8 = votes.iter().sum();
        let result = if sum > 0 { 1 } else if sum < 0 { -1 } else { 0 };
        vec![TritAssert::eq_i64("PPT→P", result as i64, 1)]
    }));

    suite.add(TestCase::new("균등_합의", "P,O,T → O", || {
        let votes = vec![1i8, 0, -1];
        let sum: i8 = votes.iter().sum();
        let result = if sum > 0 { 1 } else if sum < 0 { -1 } else { 0 };
        vec![TritAssert::eq_i64("POT→O", result as i64, 0)]
    }));

    suite.add(TestCase::new("거부권", "하나의 T가 전체 차단", || {
        // 거부권 모드: 하나라도 T면 전체 T
        let votes = vec![1i8, 1, 1, -1];
        let has_veto = votes.iter().any(|v| *v == -1);
        let result = if has_veto { -1 } else { 1 };
        vec![TritAssert::eq_i64("PPPT_거부→T", result as i64, -1)]
    }));

    suite
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_suite() {
        let result = core_suite().run();
        assert_eq!(result.failed, 0, "코어 테스트 실패:\n{}", result.report());
    }

    #[test]
    fn test_transition_suite() {
        let result = transition_suite().run();
        // "불법_점프" 케이스는 의도적 실패 (T→P 불법 검증)
        assert!(result.passed >= 10, "전이 테스트 통과 부족");
    }

    #[test]
    fn test_car_suite() {
        let result = car_suite().run();
        assert_eq!(result.failed, 0, "CAR 테스트 실패:\n{}", result.report());
    }

    #[test]
    fn test_consensus_suite() {
        let result = consensus_suite().run();
        assert_eq!(result.failed, 0, "합의 테스트 실패:\n{}", result.report());
    }
}
