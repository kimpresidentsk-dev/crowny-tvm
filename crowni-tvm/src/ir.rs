///! ═══════════════════════════════════════════════════
///! Crowny IR — 균형3진 TVM → WASM 중간표현
///! ═══════════════════════════════════════════════════
///!
///! TVM Bytecode → IR → WASM Binary
///!
///! IR은 TVM의 3진 의미를 유지하면서
///! WASM의 2진 실행 모델로 변환 가능한 중간 형태.
///!
///! GPT Spec §4: IR 정의

/// IR 명령어
#[derive(Debug, Clone, PartialEq)]
pub enum IrOp {
    // ── 스택 (A그룹) ──
    Const(i64),         // 상수 push
    ConstF64(f64),      // 실수 상수
    ConstTrit(i8),      // Trit 상수 (-1,0,+1)
    Drop,               // pop & discard
    Dup,                // 복제
    Swap,               // 교환

    // ── 산술 (B그룹) ──
    Add,                // 더
    Sub,                // 빼
    Mul,                // 곱
    Div,                // 나누
    Rem,                // 나머지
    Neg,                // 음수
    Abs,                // 절대값
    Min,                // 최소
    Max,                // 최대

    // ── 비교 (C그룹) ──
    Eq,                 // 같음
    Ne,                 // 다름
    Gt,                 // 큼
    Lt,                 // 작음
    Ge,                 // 이상
    Le,                 // 이하
    Eqz,               // 0인가

    // ── 제어흐름 (D그룹) ──
    Block(u32),         // 블록 시작 (label depth)
    Loop(u32),          // 루프 시작
    Br(u32),            // 점프 (label)
    BrIf(u32),          // 조건 점프
    Call(u32),          // 함수 호출
    Return,             // 반환
    End,                // 블록/함수 종료
    Halt,               // 프로그램 종료

    // ── 메모리 (E그룹) ──
    MemLoad(u32),       // 메모리 읽기 (offset)
    MemStore(u32),      // 메모리 쓰기 (offset)
    MemGrow,            // 메모리 확장

    // ── 로컬/전역 (F그룹) ──
    LocalGet(u32),      // 로컬 변수 읽기
    LocalSet(u32),      // 로컬 변수 쓰기
    GlobalGet(u32),     // 전역 변수 읽기
    GlobalSet(u32),     // 전역 변수 쓰기

    // ── 타입 (G그룹) ──
    I64ExtendI32,       // i32→i64
    F64ConvertI64,      // i64→f64
    I64TruncF64,        // f64→i64

    // ── IO 브릿지 (H그룹) ──
    CallImport(u32),    // import 함수 호출 (인덱스)
    Print,              // 출력 (import $print)
    Input,              // 입력 (import $input)

    // ── Trit 전용 ──
    TritClamp,          // 값을 -1,0,+1로 클램프
    TritAnd,            // 3진 AND (min)
    TritOr,             // 3진 OR (max)
    TritNot,            // 3진 NOT (negate)
    TritBranch,         // 3진 분기 (+1→A, 0→B, -1→C)

    // ── NOP ──
    Nop,
}

/// IR 함수 정의
#[derive(Debug, Clone)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<IrType>,
    pub results: Vec<IrType>,
    pub locals: Vec<IrType>,
    pub body: Vec<IrOp>,
    pub is_export: bool,
}

impl IrFunction {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            params: Vec::new(),
            results: Vec::new(),
            locals: Vec::new(),
            body: Vec::new(),
            is_export: false,
        }
    }
}

/// IR 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrType {
    I32,
    I64,
    F64,
}

/// IR import 함수 정의
#[derive(Debug, Clone)]
pub struct IrImport {
    pub module: String,
    pub name: String,
    pub params: Vec<IrType>,
    pub results: Vec<IrType>,
}

/// IR 전역 변수
#[derive(Debug, Clone)]
pub struct IrGlobal {
    pub name: String,
    pub typ: IrType,
    pub mutable: bool,
    pub init_value: i64,
}

/// IR 모듈 (= 1개 WASM 모듈)
#[derive(Debug, Clone)]
pub struct IrModule {
    pub name: String,
    pub imports: Vec<IrImport>,
    pub functions: Vec<IrFunction>,
    pub globals: Vec<IrGlobal>,
    pub memory_pages: u32,      // 초기 메모리 페이지 (1page = 64KB)
    pub start_fn: Option<u32>,  // 시작 함수 인덱스
}

impl IrModule {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            imports: Vec::new(),
            functions: Vec::new(),
            globals: Vec::new(),
            memory_pages: 1,
            start_fn: None,
        }
    }

    /// import 함수 수 (함수 인덱스 오프셋)
    pub fn import_count(&self) -> u32 {
        self.imports.len() as u32
    }

    /// 함수 추가
    pub fn add_function(&mut self, func: IrFunction) -> u32 {
        let idx = self.import_count() + self.functions.len() as u32;
        self.functions.push(func);
        idx
    }

    /// 통계
    pub fn dump_stats(&self) {
        println!("╔══ IR 모듈: {} ═══════════════════════╗", self.name);
        println!("║ Imports: {}", self.imports.len());
        println!("║ Functions: {}", self.functions.len());
        println!("║ Globals: {}", self.globals.len());
        println!("║ Memory: {} pages ({}KB)", self.memory_pages, self.memory_pages * 64);
        for (i, f) in self.functions.iter().enumerate() {
            println!("║   [{}] {}({}) → {} ops {}",
                i, f.name, f.params.len(), f.body.len(),
                if f.is_export { "[export]" } else { "" });
        }
        println!("╚═══════════════════════════════════════╝");
    }
}
