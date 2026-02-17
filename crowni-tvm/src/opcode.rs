///! 729 Opcode 맵 — 한선어 명세 v1.0 + GPT 실행 명세 통합
///!
///! 섹터 배치:
///!   0: 코어(Kernel)      4: 표현(Expression)   8: 확장(User)
///!   1: 지능(Intelligence) 5: 초월(Transcendence)
///!   2: 하드웨어(Hardware)  6: 보안(Security)
///!   3: 기억(Memory)       7: 메타(Meta)

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpcodeAddr {
    pub sector: u8,
    pub group: u8,
    pub command: u8,
}

impl OpcodeAddr {
    pub fn new(s: u8, g: u8, c: u8) -> Self {
        Self { sector: s, group: g, command: c }
    }
    /// 선형 인덱스 0..728
    pub fn linear(&self) -> u16 {
        self.sector as u16 * 81 + self.group as u16 * 9 + self.command as u16
    }
}

impl std::fmt::Display for OpcodeAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{},{})", self.sector, self.group, self.command)
    }
}

/// 명령어 효과 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Stack,    // 스택 조작
    Control,  // 제어 흐름
    Heap,     // 힙 메모리
    IO,       // 입출력
    Meta,     // 메타/시스템
    None,     // NOP
}

/// opcode 메타데이터 — GPT 명세의 계약 형식
#[derive(Debug, Clone)]
pub struct OpMeta {
    pub name_kr: &'static str,
    pub name_en: &'static str,
    pub pops: u8,
    pub pushes: u8,
    pub operands: u8,   // 추가 피연산자 수
    pub effect: Effect,
}

pub const SECTOR_NAMES: [(&str, &str); 9] = [
    ("코어",     "Kernel"),
    ("지능",     "Intelligence"),
    ("하드웨어", "Hardware"),
    ("기억",     "Memory"),
    ("표현",     "Expression"),
    ("초월",     "Transcendence"),
    ("보안",     "Security"),
    ("메타",     "Meta"),
    ("확장",     "User"),
];

pub const GROUP_NAMES_CORE: [&str; 9] = [
    "논리", "산술", "제어", "스택", "함수", "타입", "예외", "컬렉션", "접근",
];

macro_rules! op {
    ($kr:expr, $en:expr, $pop:expr, $push:expr, $oper:expr, $eff:expr) => {
        OpMeta { name_kr: $kr, name_en: $en, pops: $pop, pushes: $push, operands: $oper, effect: $eff }
    };
}

/// 코어 섹터(0) 전체 + 기타 섹터 일부 빌드
pub fn build_opcodes() -> HashMap<OpcodeAddr, OpMeta> {
    let mut m = HashMap::new();
    let s = 0u8;

    // ── G0: 논리 (3진 논리 연산) ──────────────────────
    m.insert(OpcodeAddr::new(s,0,0), op!("참",     "TRUE",    0,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,1), op!("거짓",   "FALSE",   0,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,2), op!("모름",   "UNKNOWN", 0,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,3), op!("같다",   "EQ",      2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,4), op!("다르다", "NEQ",     2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,5), op!("크다",   "GT",      2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,6), op!("작다",   "LT",      2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,7), op!("아니다", "NOT",     1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,8), op!("그리고", "AND",     2,1,0, Effect::Stack));

    // ── G1: 산술 ──────────────────────────────────────
    m.insert(OpcodeAddr::new(s,1,0), op!("더해",   "ADD",  2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,1), op!("빼",     "SUB",  2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,2), op!("곱해",   "MUL",  2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,3), op!("나눠",   "DIV",  2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,4), op!("나머지", "MOD",  2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,5), op!("음수",   "NEG",  1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,6), op!("절댓값", "ABS",  1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,7), op!("제곱",   "SQR",  1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,8), op!("제곱근", "SQRT", 1,1,0, Effect::Stack));

    // ── G2: 제어 (GPT Core 27 필수) ──────────────────
    m.insert(OpcodeAddr::new(s,2,0), op!("점프",     "JMP",   1,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,2,1), op!("조건점프", "JMPIF", 2,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,2,2), op!("호출",     "CALL",  1,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,2,3), op!("반환",     "RET",   0,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,2,4), op!("반복",     "LOOP",  2,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,2,5), op!("멈춰",     "BREAK", 0,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,2,6), op!("계속",     "CONT",  0,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,2,7), op!("종료",     "HALT",  0,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,2,8), op!("비교",     "CMP",   2,1,0, Effect::Stack));

    // ── G3: 스택 (GPT Core 27 필수) ──────────────────
    m.insert(OpcodeAddr::new(s,3,0), op!("넣어",   "PUSH",  0,1,1, Effect::Stack));
    m.insert(OpcodeAddr::new(s,3,1), op!("꺼내",   "POP",   1,0,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,3,2), op!("복사",   "DUP",   1,2,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,3,3), op!("바꿔",   "SWAP",  2,2,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,3,4), op!("비움",   "CLEAR", 0,0,0, Effect::Stack));  // N→0
    m.insert(OpcodeAddr::new(s,3,5), op!("보여줘", "PRINT", 1,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,3,6), op!("입력해", "INPUT", 0,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,3,7), op!("저장해", "STORE", 2,0,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,3,8), op!("불러와", "LOAD",  1,1,0, Effect::Heap));

    // ── G4: 함수 ─────────────────────────────────────
    m.insert(OpcodeAddr::new(s,4,0), op!("함수",     "FUNC",   0,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,4,1), op!("매개변수", "PARAM",  0,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,4,2), op!("돌려줘",   "RETURN", 1,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,4,3), op!("재귀",     "RECUR",  0,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,4,4), op!("람다",     "LAMBDA", 0,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,4,5), op!("적용해",   "APPLY",  2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,4,6), op!("묶어",     "BIND",   2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,4,7), op!("풀어",     "UNBIND", 1,0,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,4,8), op!("없다",     "NOP",    0,0,0, Effect::None));

    // ── G5: 타입 변환 ────────────────────────────────
    m.insert(OpcodeAddr::new(s,5,0), op!("정수로",   "TOINT",  1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,5,1), op!("실수로",   "TOFLT",  1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,5,2), op!("문자로",   "TOSTR",  1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,5,3), op!("트릿으로", "TOTRIT", 1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,5,4), op!("타입",     "TYPE",   1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,5,5), op!("논리로",   "TOBOOL", 1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,5,6), op!("클래스",   "CLASS",  0,0,0, Effect::Meta));
    m.insert(OpcodeAddr::new(s,5,7), op!("상속",     "INHERIT",0,0,0, Effect::Meta));
    m.insert(OpcodeAddr::new(s,5,8), op!("구현해",   "IMPL",   0,0,0, Effect::Meta));

    // ── G6: 예외 ─────────────────────────────────────
    m.insert(OpcodeAddr::new(s,6,0), op!("시도해", "TRY",    0,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,6,1), op!("잡아",   "CATCH",  0,1,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,6,2), op!("던져",   "THROW",  1,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,6,3), op!("마무리", "FINALLY",0,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,6,4), op!("확인해", "ASSERT", 1,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,6,5), op!("경고",   "WARN",   1,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,6,6), op!("오류",   "ERROR",  1,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,6,7), op!("기록",   "LOG",    1,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,6,8), op!("추적",   "TRACE",  0,1,0, Effect::Meta));

    // ── G7: 컬렉션 ──────────────────────────────────
    m.insert(OpcodeAddr::new(s,7,0), op!("배열",     "ARRAY",  1,1,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,7,1), op!("추가해",   "APPEND", 2,1,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,7,2), op!("길이",     "LEN",    1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,7,3), op!("인덱스",   "INDEX",  2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,7,4), op!("슬라이스", "SLICE",  3,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,7,5), op!("맵",       "MAP",    2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,7,6), op!("필터",     "FILTER", 2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,7,7), op!("접어",     "FOLD",   3,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,7,8), op!("정렬해",   "SORT",   1,1,0, Effect::Stack));

    // ── G8: 접근 제어 + 힙 ──────────────────────────
    m.insert(OpcodeAddr::new(s,8,0), op!("공개",   "PUBLIC",  0,0,0, Effect::Meta));
    m.insert(OpcodeAddr::new(s,8,1), op!("비공개", "PRIVATE", 0,0,0, Effect::Meta));
    m.insert(OpcodeAddr::new(s,8,2), op!("보호",   "PROTECT", 0,0,0, Effect::Meta));
    m.insert(OpcodeAddr::new(s,8,3), op!("할당",   "ALLOC",   1,1,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,8,4), op!("해제",   "FREE",    1,0,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,8,5), op!("읽어",   "HREAD",   1,1,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,8,6), op!("써",     "HWRITE",  2,0,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,8,7), op!("레지읽기","RLOAD",  0,1,1, Effect::Stack));
    m.insert(OpcodeAddr::new(s,8,8), op!("레지쓰기","RSTORE", 1,0,1, Effect::Stack));

    m
}

/// 이름(한/영) → OpcodeAddr 역방향 조회
pub fn build_name_lookup(map: &HashMap<OpcodeAddr, OpMeta>) -> HashMap<String, OpcodeAddr> {
    let mut lookup = HashMap::new();
    for (addr, meta) in map {
        lookup.insert(meta.name_kr.to_string(), *addr);
        lookup.insert(meta.name_en.to_string(), *addr);
        // 소문자 영문도 등록
        lookup.insert(meta.name_en.to_lowercase(), *addr);
    }
    lookup
}
