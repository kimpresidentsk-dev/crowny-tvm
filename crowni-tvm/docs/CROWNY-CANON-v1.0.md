# CROWNY CANON v1.0

**균형3진 실행 인프라 공식 사양서**

| 항목 | 값 |
|------|-----|
| 버전 | v1.0.0 |
| 상태 | **동결 (FROZEN)** |
| 날짜 | 2026-02-17 |
| 설계 | KPS (Han Seon) |
| 엔진 | Rust |

> **동결 선언**: 이 문서에 정의된 모든 인터페이스, 구조체, 프로토콜은 v1.x 내에서 하위호환을 보장합니다.
> 변경 시 반드시 Deprecation → Migration → Removal 3단계를 거칩니다.

---

## 1. 철학 (Philosophy)

### 1.1 균형3진법 (Balanced Ternary)

모든 상태는 3값으로 표현됩니다:

| Trit | 기호 | 한선어 | 의미 |
|------|------|--------|------|
| +1 | P | 참 | 긍정/성공/허용 |
| 0 | O | 모름 | 보류/미정/대기 |
| -1 | T | 거짓 | 부정/실패/거부 |

**핵심 원칙**:
- 2진(Yes/No)이 아닌 3진(Yes/Maybe/No) 판단
- 모든 실행 결과는 `TritResult`로 반환
- 합의는 3진 다수결

### 1.2 아키텍처 스택

```
┌─────────────────────────────────────┐
│  한선어 / 외부 앱 / SDK              │  ← 사용자 레이어
├─────────────────────────────────────┤
│  CAR (Crowny Application Runtime)   │  ← 실행 게이트
├─────────────────────────────────────┤
│  Meta-Kernel                        │  ← 핵심 엔진
│    Scheduler + Permission           │
│    Transaction + Consensus          │
│    CTP Protocol + FPGA Bridge       │
├─────────────────────────────────────┤
│  TVM (Ternary Virtual Machine)      │  ← 명령어 실행
│    729 Opcodes (9×9×9)              │
│    Stack + Heap + Frame             │
├─────────────────────────────────────┤
│  Rust Host Runtime                  │  ← 호스트
│    macOS / Linux / WASM             │
└─────────────────────────────────────┘
```

### 1.3 불변 규칙

1. **모든 앱은 CAR.submit() 경유** — 직접 Meta-Kernel 호출 금지
2. **모든 결과는 TritResult** — 예외 없음
3. **권한 검사 후 실행** — Permission Engine 통과 필수
4. **상태 불변성** — 트랜잭션 밖 상태 변경 금지

---

## 2. TritResult (표준 반환 구조체)

```rust
pub struct TritResult {
    pub state: TritState,     // P(+1), O(0), T(-1)
    pub data: ResultData,     // 반환 데이터
    pub elapsed_ms: u64,      // 실행 시간
    pub task_id: u64,         // 작업 고유 ID
}

pub enum TritState {
    Success,  // +1 (P)
    Pending,  // 0  (O)
    Failed,   // -1 (T)
}

pub enum ResultData {
    None,
    Integer(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
    Trit(i8),
    List(Vec<ResultData>),
    Map(Vec<(String, ResultData)>),
}
```

**Display 규칙**:
- `P(성공)` / `O(보류)` / `T(실패)`
- ResultData는 내용 요약 (최대 50자)

---

## 3. CAR API (Crowny Application Runtime)

### 3.1 핵심 메서드

```rust
impl CrownyRuntime {
    /// 모든 실행의 단일 진입점
    pub fn submit(
        &mut self,
        task: AppTask,
        executor: impl FnOnce(&AppTask) -> (TritState, ResultData)
    ) -> TritResult;

    /// 소스 코드 실행 (편의 메서드)
    pub fn run_source(&mut self, who: &str, source: &str) -> TritResult;

    /// WASM 컴파일 (편의 메서드)
    pub fn compile_wasm(&mut self, who: &str, source: &str) -> TritResult;
}
```

### 3.2 AppTask

```rust
pub struct AppTask {
    pub task_type: TaskType,
    pub subject: String,     // 실행 주체
    pub payload: String,     // 실행 내용
    pub params: HashMap<String, String>,
}

pub enum TaskType {
    Compile, Execute, WebRequest,
    LlmCall, DbQuery, FileIO, System,
}
```

### 3.3 접근 수준

```rust
pub enum AccessLevel {
    Public,   // 누구나
    User,     // 인증 사용자
    Admin,    // 관리자
    Kernel,   // 커널 전용
}
```

---

## 4. TVM (Ternary Virtual Machine)

### 4.1 구조

| 구성 | 설명 | 크기 |
|------|------|------|
| Stack | 값 스택 | 가변 |
| Heap | 키-값 저장소 | 가변 |
| Frames | 콜 프레임 | 최대 256 |
| PC | 프로그램 카운터 | u32 |

### 4.2 명령어

```rust
pub struct Instruction {
    pub addr: OpcodeAddr,      // (섹터, 그룹, 명령)
    pub operands: Vec<Value>,  // 피연산자
}

pub struct OpcodeAddr {
    pub sector: u8,   // 0~8
    pub group: u8,    // 0~8
    pub command: u8,  // 0~8
}
```

### 4.3 값 타입

```rust
pub enum Value {
    None,
    Int(i64),
    Float(f64),
    Bool(bool),
    Trit(i8),       // -1, 0, +1
    Str(String),
    Nil,
}
```

---

## 5. 729 Opcode Map (9×9×9)

### 5.1 섹터 배치

| ID | 이름 | 영문 | 활성 | 용도 |
|----|------|------|------|------|
| 0 | 코어 | Kernel | 80 | 논리, 산술, 제어, 스택, 함수, 타입, 예외, 컬렉션, 접근 |
| 1 | 지능 | Intelligence | 45 | LLM, 추론, 텐서, 학습, 자연어 |
| 2 | 하드웨어 | Hardware | 18 | FPGA, GPIO |
| 3 | 기억 | Memory | 18 | 캐시, GC |
| 4 | 표현 | Expression | 18 | 문자열, 정규식, JSON |
| 5 | 초월 | Transcendence | 9 | 해시, 암호화 |
| 6 | 보안 | Security | 13 | 인증, 감사 |
| 7 | 메타 | Meta | 9 | 디버그, 프로파일 |
| 8 | 확장 | User | 9 | 플러그인, FFI, WASM |

### 5.2 섹터 0: 코어 (주요 명령어)

**G0: 3진 논리**

| (S,G,C) | 한선어 | 영문 | Pop | Push | 효과 |
|---------|--------|------|-----|------|------|
| 0,0,0 | 참 | TRUE | 0 | 1 | Stack |
| 0,0,1 | 거짓 | FALSE | 0 | 1 | Stack |
| 0,0,2 | 모름 | UNKNOWN | 0 | 1 | Stack |
| 0,0,3 | 같다 | EQ | 2 | 1 | Stack |
| 0,0,4 | 다르다 | NEQ | 2 | 1 | Stack |
| 0,0,5 | 크다 | GT | 2 | 1 | Stack |
| 0,0,6 | 작다 | LT | 2 | 1 | Stack |
| 0,0,7 | 아니다 | NOT | 1 | 1 | Stack |
| 0,0,8 | 그리고 | AND | 2 | 1 | Stack |

**G1: 산술**

| (S,G,C) | 한선어 | 영문 | Pop | Push |
|---------|--------|------|-----|------|
| 0,1,0 | 더해 | ADD | 2 | 1 |
| 0,1,1 | 빼 | SUB | 2 | 1 |
| 0,1,2 | 곱해 | MUL | 2 | 1 |
| 0,1,3 | 나눠 | DIV | 2 | 1 |
| 0,1,4 | 나머지 | MOD | 2 | 1 |
| 0,1,5 | 올림 | CEIL | 1 | 1 |
| 0,1,6 | 내림 | FLOOR | 1 | 1 |
| 0,1,7 | 절대 | ABS | 1 | 1 |
| 0,1,8 | 제곱근 | SQRT | 1 | 1 |

**G2: 제어 흐름**

| (S,G,C) | 한선어 | 영문 | 설명 |
|---------|--------|------|------|
| 0,2,0 | 가라 | JUMP | 무조건 점프 |
| 0,2,1 | 만약가라 | JUMP_IF | P이면 점프 |
| 0,2,2 | 불러 | CALL | 함수 호출 |
| 0,2,3 | 돌아가 | RET | 함수 반환 |
| 0,2,4 | 반복해 | LOOP | 루프 |
| 0,2,5 | 끊어 | BREAK | 루프 탈출 |
| 0,2,6 | 무시 | NOP | 아무것도 안 함 |
| 0,2,7 | 종료 | HALT | 프로그램 종료 |
| 0,2,8 | 삼분기 | BRANCH3 | P/O/T 3갈래 분기 |

**G3: 스택 조작**

| (S,G,C) | 한선어 | 영문 | 설명 |
|---------|--------|------|------|
| 0,3,0 | 넣어 | PUSH | 값을 스택에 |
| 0,3,1 | 빼내 | POP | 스택에서 제거 |
| 0,3,2 | 복사 | DUP | 맨 위 복사 |
| 0,3,3 | 바꿔 | SWAP | 위 2개 교환 |
| 0,3,4 | 돌려 | ROT | 위 3개 회전 |
| 0,3,5 | 보여줘 | PRINT | 출력 |
| 0,3,6 | 깊이 | DEPTH | 스택 깊이 |
| 0,3,7 | 저장해 | STORE | 슬롯에 저장 |
| 0,3,8 | 불러와 | LOAD | 슬롯에서 로드 |

### 5.3 섹터 1: 지능 (핵심 명령어)

| (S,G,C) | 한선어 | 영문 | 용도 |
|---------|--------|------|------|
| 1,0,0 | 질문해 | LLM_ASK | LLM 질의 |
| 1,0,1 | 요약해 | LLM_SUMMARY | 요약 |
| 1,0,2 | 번역해 | LLM_TRANSLATE | 번역 |
| 1,0,3 | 분류해 | LLM_CLASSIFY | 분류 |
| 1,0,5 | 임베딩 | LLM_EMBED | 벡터화 |
| 1,2,1 | 행렬곱 | MATMUL | 행렬 연산 |
| 1,2,6 | 소프트맥스 | SOFTMAX | 확률 분포 |
| 1,4,3 | 감정분석 | SENTIMENT | NLP |

---

## 6. CTP (Crowny Trit Protocol)

### 6.1 9-Trit 헤더

```
[0] 상태(State)      : P/O/T
[1] 권한(Permission)  : P/O/T
[2] 합의(Consensus)   : P/O/T
[3] 트랜잭션(Txn)     : P/O/T
[4] 라우팅(Routing)   : P/O/T
[5] 예약1(Reserved)   : P/O/T
[6] 예약2(Reserved)   : P/O/T
[7] 예약3(Reserved)   : P/O/T
[8] 예약4(Reserved)   : P/O/T
```

### 6.2 HTTP 확장

```
X-Crowny-Trit: PPPOOOOOO
X-Crowny-Version: 1.0
Content-Type: application/crowny+json
```

### 6.3 판정 규칙

- 하나라도 T(-1) → **전체 실패** (하향 안정성)
- 모두 P(+1) → **성공**
- 그 외 → **보류**

---

## 7. .크라운 바이트코드 포맷

### 7.1 파일 구조

```
Offset  Size  설명
──────  ────  ──────────────────
0x00    4     Magic: 0xCB 0x33 0xCB 0x33
0x04    1     Version: 0x01
0x05    1     Flags: 0x00
0x06    4     Instruction Count (u32 LE)
0x0A    N     Instructions...
```

### 7.2 명령어 인코딩

```
Offset  Size  설명
──────  ────  ──────────────────
+0      1     Sector (0~8)
+1      1     Group (0~8)
+2      1     Command (0~8)
+3      1     Operand Count
+4      N     Operands (tagged)
```

### 7.3 값 태그

| Tag | 타입 | 크기 |
|-----|------|------|
| 0x00 | None | 0 |
| 0x01 | Int(i64) | 8 |
| 0x02 | Float(f64) | 8 |
| 0x03 | Bool | 1 |
| 0x04 | Trit(i8) | 1 |
| 0x05 | Str | 2+N (u16 LE 길이 + UTF-8) |
| 0x06 | Nil | 0 |

---

## 8. TVM IR (중간 표현)

### 8.1 IR 명령

```rust
pub enum IrOp {
    Push(Value),
    Pop,
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Gt,
    Not, And,
    TritTrue, TritFalse, TritUnknown,
    Load(u32), Store(u32),
    Jump(u32), JumpIf(u32),
    Call(u32), Ret,
    Loop(u32), Break,
    Branch3 { p: u32, o: u32, t: u32 },
    Print,
    Dup, Swap, Rot,
    Nop, Halt,
    Ceil, Floor, Abs, Sqrt,
}
```

### 8.2 WASM 변환

IR → WASM 매핑:
- `Push(Int)` → `i64.const N`
- `Add` → `i64.add`
- `Print` → `call $print`
- `Halt` → `return`

---

## 9. 한선어 문법 (Crowny WebScript)

### 9.1 키워드

| 한국어 | 영문 | 용도 |
|--------|------|------|
| 값 | val | 리터럴 push |
| 변수 | var/let | 변수 선언 |
| 만약 | if | 조건 분기 |
| 아니면 | else | else 분기 |
| 보류 | neutral | 3진 중간 분기 |
| 반복 | loop/repeat | 루프 |
| 함수 | func/fn | 함수 정의 |
| 반환 | return | 함수 반환 |
| 끝 | end | 종료 |
| 보여줘 | print | 출력 |
| 질문해 | ask/llm | LLM 호출 |
| 더 | add | 덧셈 |
| 빼 | sub | 뺄셈 |
| 곱 | mul | 곱셈 |
| 나눠 | div | 나눗셈 |
| 참 | P | Trit +1 |
| 모름 | O | Trit 0 |
| 거짓 | T | Trit -1 |

### 9.2 문법 예시

```crowny
; 3진 AI 판단
변수 점수 = 85

만약 {
    질문해 "이 점수가 합격인가?"
    보여줘
} 보류 {
    질문해 "추가 심사 필요"
    보여줘
} 아니면 {
    값 0
    보여줘
}

; 함수 정의
함수 계산 {
    값 7
    값 3
    곱
    보여줘
}

; LLM 호출
질문해 "균형3진법의 장점은?"
보여줘

끝
```

---

## 10. 패키지 시스템 (CPM)

### 10.1 패키지 구조

```
패키지이름/
├── crowny.toml       # 패키지 메타데이터
├── src/
│   └── main.cws      # 한선어 소스
├── tests/
│   └── test.cws      # 테스트
└── docs/
    └── README.md
```

### 10.2 crowny.toml

```toml
[package]
name = "my-app"
version = "1.0.0"
author = "KPS"
license = "MIT"
trit-version = "1.0"

[dependencies]
crowny-ai = "0.1"
crowny-web = "0.2"

[permissions]
llm = "user"
network = "admin"
file = "user"
```

### 10.3 CPM 명령어

```bash
cpm init                    # 새 패키지
cpm install <패키지>         # 설치
cpm publish                 # 저장소 게시
cpm build                   # 빌드
cpm test                    # 테스트 실행
cpm run                     # 실행
```

---

## 11. 테스트 프레임워크

### 11.1 Trit 테스트 구조

```rust
pub struct TritTest {
    pub name: String,
    pub description: String,
    pub source: String,           // 한선어 소스
    pub expected_state: TritState,
    pub expected_data: Option<ResultData>,
}
```

### 11.2 테스트 실행

```bash
crowni-tvm test              # 전체 테스트
crowni-tvm test --filter ai  # 필터링
crowni-tvm test --verbose    # 상세 출력
```

### 11.3 어서션

```
trit_assert P               → state == Success
trit_assert O               → state == Pending
trit_assert T               → state == Failed
trit_assert_data "hello"    → data == Text("hello")
trit_assert_between 1 100   → 1 <= data <= 100
```

---

## 12. 디버거

### 12.1 명령어

| 명령 | 설명 |
|------|------|
| step / s | 한 명령어 실행 |
| continue / c | 중단점까지 실행 |
| break N | N번 명령어에 중단점 |
| stack | 스택 내용 표시 |
| heap | 힙 내용 표시 |
| trit | 현재 Trit 상태 |
| info | VM 정보 |
| backtrace / bt | 콜 스택 |
| watch SLOT | 변수 감시 |

### 12.2 사용법

```bash
crowni-tvm debug 프로그램.hsn
[DBG]> break 5
[DBG]> continue
[DBG]> stack
  [0] Int(42)
  [1] Trit(P)
[DBG]> step
```

---

## 13. 영속화 (Trit Store)

### 13.1 스냅샷

```rust
pub struct TritSnapshot {
    pub version: u32,
    pub timestamp: u64,
    pub trit_values: HashMap<String, TritValue>,
    pub metadata: HashMap<String, String>,
    pub checksum: u64,
}
```

### 13.2 저장소 경로

```
~/.crowny/
├── store/
│   └── default.trit        # 기본 저장소
├── snapshots/
│   └── snap_1708123456.trit
├── logs/
│   └── events.tlog
└── config.toml
```

---

## 14. 이벤트 로그 (Trit Event Log)

### 14.1 이벤트 타입

```rust
pub enum TritEventType {
    TaskSubmit,     // 작업 제출
    TaskComplete,   // 작업 완료
    StateChange,    // 상태 전이
    PermCheck,      // 권한 확인
    ConsVote,       // 합의 투표
    ConsResult,     // 합의 결과
    Error,          // 오류
    Warning,        // 경고
    Debug,          // 디버그
}
```

### 14.2 로그 포맷

```
[2026-02-17T12:00:00Z] [P] TaskComplete task#42 "컴파일" 15ms
[2026-02-17T12:00:01Z] [T] Error task#43 "권한 부족"
[2026-02-17T12:00:02Z] [O] ConsVote "노드A=P, 노드B=O, 노드C=P"
```

---

## 15. 버전 호환성

### 15.1 시맨틱 버전

```
MAJOR.MINOR.PATCH

MAJOR: 호환 깨지는 변경 (v2.0 시)
MINOR: 하위 호환 기능 추가
PATCH: 버그 수정
```

### 15.2 v1.x 보장 사항

- TritResult 구조 변경 없음
- CAR.submit() 시그니처 변경 없음
- CTP 9-Trit 헤더 변경 없음
- .크라운 바이트코드 포맷 변경 없음 (확장만 허용)
- 729 Opcode 주소 변경 없음 (예약 슬롯 활성화만 허용)
- Value 열거형 확장만 허용 (제거 금지)

---

## 부록 A: 파일 확장자

| 확장자 | 용도 |
|--------|------|
| .cws | 한선어 소스 (Crowny WebScript) |
| .hsn | 한선어 소스 (별칭) |
| .크라운 | TVM 바이트코드 |
| .trit | Trit 저장소 |
| .tlog | Trit 이벤트 로그 |
| .wasm | WebAssembly 출력 |

## 부록 B: 포트 번호

| 포트 | 용도 |
|------|------|
| 7293 | Crowny 웹서버 기본 |
| 7294 | 디버거 |
| 7295 | CPM 레지스트리 |

## 부록 C: 매직 넘버

| 값 | 용도 |
|----|------|
| 0xCB33CB33 | .크라운 파일 헤더 |
| 0x00617373 | WASM 매직 |
| 0x54524954 | .trit 저장소 헤더 |

---

**© 2026 CROWNIN Project. All specifications frozen under Canon v1.0.**
