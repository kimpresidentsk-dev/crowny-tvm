///! CROWNIN TVM 실행기
///! GPT 명세: 스택 기반 VM + 힙(Arena) + 레지스터 9개
///!
///! struct TVM {
///!     stack: Vec<Value>,
///!     heap: Heap,
///!     registers: [Value; 9],
///!     ip: usize,
///!     program: Vec<Instruction>,
///!     halted: bool,
///! }

use std::collections::HashMap;
use std::io::{self, Write};

use crate::trit::Trit;
use crate::value::Value;
use crate::heap::Heap;
use crate::opcode::{OpcodeAddr, OpMeta, build_opcodes, build_name_lookup};

// ─────────────────────────────────────────────
// Error
// ─────────────────────────────────────────────

#[derive(Debug)]
pub enum VmError {
    StackUnderflow(String),
    TypeError(String),
    DivisionByZero,
    InvalidOpcode(u8, u8, u8),
    Halted,
    HeapError(String),
    Custom(String),
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::StackUnderflow(op) => write!(f, "[스택부족] '{}'", op),
            VmError::TypeError(msg) => write!(f, "[타입오류] {}", msg),
            VmError::DivisionByZero => write!(f, "[오류] 0으로 나눌 수 없음"),
            VmError::InvalidOpcode(s, g, c) => write!(f, "[알수없는명령] ({},{},{})", s, g, c),
            VmError::Halted => write!(f, "[종료]"),
            VmError::HeapError(msg) => write!(f, "[힙오류] {}", msg),
            VmError::Custom(msg) => write!(f, "[오류] {}", msg),
        }
    }
}

// ─────────────────────────────────────────────
// Instruction (GPT 명세)
// ─────────────────────────────────────────────

/// Instruction = opcode(6-trit) + operands
#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: [i8; 6],          // 균형3진 6트릿 (-1,0,+1)
    pub addr: OpcodeAddr,         // 디코딩된 (sector,group,command)
    pub operands: Vec<Value>,     // 추가 피연산자
}

impl Instruction {
    /// OpcodeAddr + 선택적 피연산자로 생성
    pub fn from_addr(addr: OpcodeAddr, operands: Vec<Value>) -> Self {
        use crate::trit::Word6;
        let w = Word6::encode_opcode(addr.sector, addr.group, addr.command);
        let opcode = [
            w.trits[0].to_i8(), w.trits[1].to_i8(), w.trits[2].to_i8(),
            w.trits[3].to_i8(), w.trits[4].to_i8(), w.trits[5].to_i8(),
        ];
        Self { opcode, addr, operands }
    }
}

// ─────────────────────────────────────────────
// Call Frame
// ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub return_ip: usize,
    pub base_sp: usize,  // 호출 시 스택 깊이
}

// ─────────────────────────────────────────────
// TVM — The Virtual Machine
// ─────────────────────────────────────────────

pub struct TVM {
    /// 데이터 스택
    pub stack: Vec<Value>,
    /// Arena 힙
    pub heap: Heap,
    /// 레지스터 9개 (R0..R8)
    pub registers: [Value; 9],
    /// 명령어 포인터 (Instruction Pointer)
    pub ip: usize,
    /// 프로그램
    pub program: Vec<Instruction>,
    /// 종료 플래그
    pub halted: bool,
    /// 호출 스택
    pub call_stack: Vec<CallFrame>,
    /// 전역 변수
    pub globals: HashMap<String, Value>,
    /// opcode 메타
    pub opcodes: HashMap<OpcodeAddr, OpMeta>,
    /// 이름→opcode 역조회
    pub name_lookup: HashMap<String, OpcodeAddr>,
    /// 디버그 모드
    pub debug: bool,
    /// 실행된 명령어 수 (프로파일링)
    pub cycles: u64,
}

impl TVM {
    pub fn new() -> Self {
        let opcodes = build_opcodes();
        let name_lookup = build_name_lookup(&opcodes);
        Self {
            stack: Vec::with_capacity(1024),
            heap: Heap::new(),
            registers: std::array::from_fn(|_| Value::Nil),
            ip: 0,
            program: Vec::new(),
            halted: false,
            call_stack: Vec::new(),
            globals: HashMap::new(),
            opcodes,
            name_lookup,
            debug: false,
            cycles: 0,
        }
    }

    /// 프로그램 로드
    pub fn load(&mut self, program: Vec<Instruction>) {
        self.program = program;
        self.ip = 0;
        self.halted = false;
        self.stack.clear();
        self.call_stack.clear();
        self.cycles = 0;
    }

    // ── 스택 헬퍼 ──

    fn pop(&mut self, op: &str) -> Result<Value, VmError> {
        self.stack.pop().ok_or_else(|| VmError::StackUnderflow(op.into()))
    }

    fn pop2_int(&mut self, op: &str) -> Result<(i64, i64), VmError> {
        let b = self.pop(op)?;
        let a = self.pop(op)?;
        let ai = a.as_int().ok_or_else(|| VmError::TypeError(format!("{}: 정수 필요, got {}", op, a.type_name_kr())))?;
        let bi = b.as_int().ok_or_else(|| VmError::TypeError(format!("{}: 정수 필요, got {}", op, b.type_name_kr())))?;
        Ok((ai, bi))
    }

    // ── 메인 실행 루프 (GPT 명세 §7) ──

    pub fn run(&mut self) -> Result<(), VmError> {
        // GPT: while !vm.halted { let inst = vm.program[vm.ip]; vm.ip += 1; match ... }
        while !self.halted {
            if self.ip >= self.program.len() {
                self.halted = true;
                break;
            }

            let inst = self.program[self.ip].clone();
            self.ip += 1;
            self.cycles += 1;

            if self.debug {
                let name = self.opcodes.get(&inst.addr).map(|m| m.name_kr).unwrap_or("???");
                eprintln!("[IP:{:04}] {} {} | 스택:{} 힙:{}",
                    self.ip - 1, inst.addr, name, self.stack.len(), self.heap.alive_count());
            }

            self.execute(&inst)?;
        }

        if self.debug {
            eprintln!("[VM 종료] 총 {}사이클 실행", self.cycles);
        }
        Ok(())
    }

    /// 단일 스텝 실행
    pub fn step(&mut self) -> Result<bool, VmError> {
        if self.halted || self.ip >= self.program.len() {
            self.halted = true;
            return Ok(false);
        }
        let inst = self.program[self.ip].clone();
        self.ip += 1;
        self.cycles += 1;
        self.execute(&inst)?;
        Ok(!self.halted)
    }

    // ── 명령어 디스패치 ──

    fn execute(&mut self, inst: &Instruction) -> Result<(), VmError> {
        let (s, g, c) = (inst.addr.sector, inst.addr.group, inst.addr.command);

        match s {
            0 => self.exec_core(g, c, &inst.operands),
            // 섹터 1~8: 미래 확장. 현재는 NOP.
            _ => {
                // GPT 명세 §9: Reserved → NOP (pop=0 push=0 effect=None)
                Ok(())
            }
        }
    }

    // ── 섹터 0: 코어 실행 ──

    fn exec_core(&mut self, g: u8, c: u8, operands: &[Value]) -> Result<(), VmError> {
        match (g, c) {
            // ════════════════════════════════════════
            // G0: 논리
            // ════════════════════════════════════════
            (0, 0) => { self.stack.push(Value::Trit(Trit::P)); }       // 참
            (0, 1) => { self.stack.push(Value::Trit(Trit::T)); }       // 거짓
            (0, 2) => { self.stack.push(Value::Trit(Trit::O)); }       // 모름

            (0, 3) => { // 같다 EQ
                let b = self.pop("같다")?;
                let a = self.pop("같다")?;
                let r = match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => if x == y { Trit::P } else { Trit::T },
                    (Value::Float(x), Value::Float(y)) => if (x - y).abs() < f64::EPSILON { Trit::P } else { Trit::T },
                    (Value::Str(x), Value::Str(y)) => if x == y { Trit::P } else { Trit::T },
                    (Value::Trit(x), Value::Trit(y)) => if x == y { Trit::P } else { Trit::T },
                    (Value::Bool(x), Value::Bool(y)) => if x == y { Trit::P } else { Trit::T },
                    (Value::Nil, Value::Nil) => Trit::P,
                    _ => Trit::O, // 타입 불일치 → 모름
                };
                self.stack.push(Value::Trit(r));
            }
            (0, 4) => { // 다르다 NEQ
                let b = self.pop("다르다")?;
                let a = self.pop("다르다")?;
                let r = match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => if x != y { Trit::P } else { Trit::T },
                    (Value::Str(x), Value::Str(y)) => if x != y { Trit::P } else { Trit::T },
                    _ => Trit::O,
                };
                self.stack.push(Value::Trit(r));
            }
            (0, 5) => { // 크다 GT → 3진 결과: P=크다 O=같다 T=작다
                let (a, b) = self.pop2_int("크다")?;
                self.stack.push(Value::Trit(
                    if a > b { Trit::P } else if a == b { Trit::O } else { Trit::T }
                ));
            }
            (0, 6) => { // 작다 LT
                let (a, b) = self.pop2_int("작다")?;
                self.stack.push(Value::Trit(
                    if a < b { Trit::P } else if a == b { Trit::O } else { Trit::T }
                ));
            }
            (0, 7) => { // 아니다 NOT
                let a = self.pop("아니다")?;
                self.stack.push(Value::Trit(a.to_trit().not()));
            }
            (0, 8) => { // 그리고 AND
                let b = self.pop("그리고")?;
                let a = self.pop("그리고")?;
                self.stack.push(Value::Trit(a.to_trit().and(b.to_trit())));
            }

            // ════════════════════════════════════════
            // G1: 산술
            // ════════════════════════════════════════
            (1, 0) => { // 더해 ADD
                let b = self.pop("더해")?;
                let a = self.pop("더해")?;
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x + y)),
                    (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x + y)),
                    (Value::Int(x), Value::Float(y)) => self.stack.push(Value::Float(*x as f64 + y)),
                    (Value::Float(x), Value::Int(y)) => self.stack.push(Value::Float(x + *y as f64)),
                    (Value::Str(x), Value::Str(y)) => self.stack.push(Value::Str(format!("{}{}", x, y))),
                    _ => return Err(VmError::TypeError("더해: 수치/문자열 필요".into())),
                }
            }
            (1, 1) => { // 빼 SUB
                let b = self.pop("빼")?;
                let a = self.pop("빼")?;
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x - y)),
                    (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x - y)),
                    (Value::Int(x), Value::Float(y)) => self.stack.push(Value::Float(*x as f64 - y)),
                    (Value::Float(x), Value::Int(y)) => self.stack.push(Value::Float(x - *y as f64)),
                    _ => return Err(VmError::TypeError("빼: 수치 필요".into())),
                }
            }
            (1, 2) => { // 곱해 MUL
                let b = self.pop("곱해")?;
                let a = self.pop("곱해")?;
                match (&a, &b) {
                    (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x * y)),
                    (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x * y)),
                    (Value::Int(x), Value::Float(y)) => self.stack.push(Value::Float(*x as f64 * y)),
                    (Value::Float(x), Value::Int(y)) => self.stack.push(Value::Float(x * *y as f64)),
                    _ => return Err(VmError::TypeError("곱해: 수치 필요".into())),
                }
            }
            (1, 3) => { // 나눠 DIV
                let b = self.pop("나눠")?;
                let a = self.pop("나눠")?;
                match (&a, &b) {
                    (Value::Int(_), Value::Int(0)) => return Err(VmError::DivisionByZero),
                    (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x / y)),
                    (Value::Float(_, ), Value::Float(y)) if *y == 0.0 => return Err(VmError::DivisionByZero),
                    (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x / y)),
                    (Value::Int(x), Value::Float(y)) if *y == 0.0 => return Err(VmError::DivisionByZero),
                    (Value::Int(x), Value::Float(y)) => self.stack.push(Value::Float(*x as f64 / y)),
                    (Value::Float(x), Value::Int(0)) => return Err(VmError::DivisionByZero),
                    (Value::Float(x), Value::Int(y)) => self.stack.push(Value::Float(x / *y as f64)),
                    _ => return Err(VmError::TypeError("나눠: 수치 필요".into())),
                }
            }
            (1, 4) => { // 나머지 MOD
                let (a, b) = self.pop2_int("나머지")?;
                if b == 0 { return Err(VmError::DivisionByZero); }
                self.stack.push(Value::Int(a % b));
            }
            (1, 5) => { // 음수 NEG
                let a = self.pop("음수")?;
                match a {
                    Value::Int(n) => self.stack.push(Value::Int(-n)),
                    Value::Float(f) => self.stack.push(Value::Float(-f)),
                    _ => return Err(VmError::TypeError("음수: 수치 필요".into())),
                }
            }
            (1, 6) => { // 절댓값 ABS
                let a = self.pop("절댓값")?;
                match a {
                    Value::Int(n) => self.stack.push(Value::Int(n.abs())),
                    Value::Float(f) => self.stack.push(Value::Float(f.abs())),
                    _ => return Err(VmError::TypeError("절댓값: 수치 필요".into())),
                }
            }
            (1, 7) => { // 제곱 SQR
                let a = self.pop("제곱")?;
                match a {
                    Value::Int(n) => self.stack.push(Value::Int(n * n)),
                    Value::Float(f) => self.stack.push(Value::Float(f * f)),
                    _ => return Err(VmError::TypeError("제곱: 수치 필요".into())),
                }
            }
            (1, 8) => { // 제곱근 SQRT
                let a = self.pop("제곱근")?;
                let f = a.as_float().ok_or_else(|| VmError::TypeError("제곱근: 수치 필요".into()))?;
                self.stack.push(Value::Float(f.sqrt()));
            }

            // ════════════════════════════════════════
            // G2: 제어 (GPT Core 27 필수)
            // ════════════════════════════════════════
            (2, 0) => { // 점프 JMP — pop addr, ip = addr
                let target = self.pop("점프")?;
                let addr = target.as_addr().ok_or_else(|| VmError::TypeError("점프: 주소 필요".into()))?;
                self.ip = addr;
            }
            (2, 1) => { // 조건점프 JMPIF — pop cond, pop addr. if cond→P then ip=addr
                let addr_v = self.pop("조건점프")?;
                let cond = self.pop("조건점프")?;
                if cond.to_trit() == Trit::P {
                    let addr = addr_v.as_addr().ok_or_else(|| VmError::TypeError("조건점프: 주소 필요".into()))?;
                    self.ip = addr;
                }
            }
            (2, 2) => { // 호출 CALL — pop addr, push frame, ip = addr
                let target = self.pop("호출")?;
                let addr = target.as_addr().ok_or_else(|| VmError::TypeError("호출: 주소 필요".into()))?;
                self.call_stack.push(CallFrame {
                    return_ip: self.ip,
                    base_sp: self.stack.len(),
                });
                self.ip = addr;
            }
            (2, 3) => { // 반환 RET — pop frame, ip = return_ip
                if let Some(frame) = self.call_stack.pop() {
                    self.ip = frame.return_ip;
                }
                // 호출 스택 비었으면 무시 (최상위)
            }
            (2, 4) => { // 반복 LOOP — pop cond, pop addr. if cond!=T then ip=addr
                let addr_v = self.pop("반복")?;
                let cond = self.pop("반복")?;
                if cond.to_trit() != Trit::T {
                    let addr = addr_v.as_addr().ok_or_else(|| VmError::TypeError("반복: 주소 필요".into()))?;
                    self.ip = addr;
                }
            }
            (2, 5) => { /* 멈춰 BREAK — 추후 루프 컨텍스트와 연동 */ }
            (2, 6) => { /* 계속 CONT */ }
            (2, 7) => { // 종료 HALT
                self.halted = true;
            }
            (2, 8) => { // 비교 CMP — pop a, pop b, push trit(a <=> b)
                let (a, b) = self.pop2_int("비교")?;
                self.stack.push(Value::Trit(
                    if a > b { Trit::P } else if a == b { Trit::O } else { Trit::T }
                ));
            }

            // ════════════════════════════════════════
            // G3: 스택 (GPT Core 27 필수)
            // ════════════════════════════════════════
            (3, 0) => { // 넣어 PUSH — operands[0] → stack
                let val = operands.first().cloned().unwrap_or(Value::Nil);
                self.stack.push(val);
            }
            (3, 1) => { // 꺼내 POP
                self.pop("꺼내")?;
            }
            (3, 2) => { // 복사 DUP
                let a = self.pop("복사")?;
                self.stack.push(a.clone());
                self.stack.push(a);
            }
            (3, 3) => { // 바꿔 SWAP
                let b = self.pop("바꿔")?;
                let a = self.pop("바꿔")?;
                self.stack.push(b);
                self.stack.push(a);
            }
            (3, 4) => { // 비움 CLEAR
                self.stack.clear();
            }
            (3, 5) => { // 보여줘 PRINT
                let a = self.pop("보여줘")?;
                println!("{}", a);
            }
            (3, 6) => { // 입력해 INPUT
                print!("입력> ");
                io::stdout().flush().unwrap_or(());
                let mut buf = String::new();
                io::stdin().read_line(&mut buf).unwrap_or(0);
                let t = buf.trim().to_string();
                if let Ok(n) = t.parse::<i64>() {
                    self.stack.push(Value::Int(n));
                } else if let Ok(f) = t.parse::<f64>() {
                    self.stack.push(Value::Float(f));
                } else {
                    self.stack.push(Value::Str(t));
                }
            }
            (3, 7) => { // 저장해 STORE — pop value, pop name → globals
                let val = self.pop("저장해")?;
                let name = self.pop("저장해")?;
                if let Value::Str(key) = name {
                    self.globals.insert(key, val);
                } else {
                    return Err(VmError::TypeError("저장해: 이름은 문자열".into()));
                }
            }
            (3, 8) => { // 불러와 LOAD — pop name → push globals[name]
                let name = self.pop("불러와")?;
                if let Value::Str(key) = name {
                    let val = self.globals.get(&key).cloned().unwrap_or(Value::Nil);
                    self.stack.push(val);
                } else {
                    return Err(VmError::TypeError("불러와: 이름은 문자열".into()));
                }
            }

            // ════════════════════════════════════════
            // G4: 함수
            // ════════════════════════════════════════
            (4, 8) => { /* NOP 없다 */ }

            // ════════════════════════════════════════
            // G5: 타입 변환
            // ════════════════════════════════════════
            (5, 0) => { // 정수로
                let a = self.pop("정수로")?;
                match a {
                    Value::Int(_) => self.stack.push(a),
                    Value::Float(f) => self.stack.push(Value::Int(f as i64)),
                    Value::Str(ref s) => self.stack.push(Value::Int(s.parse::<i64>().unwrap_or(0))),
                    Value::Trit(t) => self.stack.push(Value::Int(t.to_i8() as i64)),
                    Value::Bool(b) => self.stack.push(Value::Int(if b { 1 } else { 0 })),
                    _ => self.stack.push(Value::Int(0)),
                }
            }
            (5, 1) => { // 실수로
                let a = self.pop("실수로")?;
                let f = a.as_float().unwrap_or(0.0);
                self.stack.push(Value::Float(f));
            }
            (5, 2) => { // 문자로
                let a = self.pop("문자로")?;
                self.stack.push(Value::Str(format!("{}", a)));
            }
            (5, 3) => { // 트릿으로
                let a = self.pop("트릿으로")?;
                self.stack.push(Value::Trit(a.to_trit()));
            }
            (5, 4) => { // 타입
                let a = self.pop("타입")?;
                self.stack.push(Value::Str(a.type_name_kr().to_string()));
            }
            (5, 5) => { // 논리로
                let a = self.pop("논리로")?;
                self.stack.push(Value::Bool(a.as_bool()));
            }

            // ════════════════════════════════════════
            // G6: 예외
            // ════════════════════════════════════════
            (6, 2) => { // 던져 THROW
                let msg = self.pop("던져")?;
                return Err(VmError::Custom(format!("{}", msg)));
            }
            (6, 6) => { // 오류 ERROR
                let msg = self.pop("오류")?;
                return Err(VmError::Custom(format!("{}", msg)));
            }
            (6, 7) => { // 기록 LOG
                let msg = self.pop("기록")?;
                eprintln!("[LOG] {}", msg);
            }

            // ════════════════════════════════════════
            // G7: 컬렉션
            // ════════════════════════════════════════
            (7, 2) => { // 길이 LEN
                let a = self.pop("길이")?;
                let len = match &a {
                    Value::Str(s) => s.chars().count() as i64,
                    Value::Array(arr) => arr.len() as i64,
                    _ => 0,
                };
                self.stack.push(Value::Int(len));
            }

            // ════════════════════════════════════════
            // G8: 접근/힙/레지스터
            // ════════════════════════════════════════
            (8, 3) => { // 할당 ALLOC — pop value → heap, push addr
                let val = self.pop("할당")?;
                let addr = self.heap.alloc(val);
                self.stack.push(Value::Addr(addr));
            }
            (8, 4) => { // 해제 FREE — pop addr → heap.free
                let a = self.pop("해제")?;
                let addr = a.as_addr().ok_or_else(|| VmError::TypeError("해제: 주소 필요".into()))?;
                if !self.heap.free(addr) {
                    return Err(VmError::HeapError(format!("해제 실패: &{}", addr)));
                }
            }
            (8, 5) => { // 읽어 HREAD — pop addr → push heap[addr]
                let a = self.pop("읽어")?;
                let addr = a.as_addr().ok_or_else(|| VmError::TypeError("읽어: 주소 필요".into()))?;
                let val = self.heap.get(addr).cloned()
                    .ok_or_else(|| VmError::HeapError(format!("읽기 실패: &{}", addr)))?;
                self.stack.push(val);
            }
            (8, 6) => { // 써 HWRITE — pop value, pop addr → heap[addr] = value
                let val = self.pop("써")?;
                let a = self.pop("써")?;
                let addr = a.as_addr().ok_or_else(|| VmError::TypeError("써: 주소 필요".into()))?;
                if !self.heap.set(addr, val) {
                    return Err(VmError::HeapError(format!("쓰기 실패: &{}", addr)));
                }
            }
            (8, 7) => { // 레지읽기 RLOAD — operands[0]=레지스터번호, push registers[n]
                let idx = operands.first()
                    .and_then(|v| v.as_int())
                    .unwrap_or(0) as usize;
                if idx < 9 {
                    self.stack.push(self.registers[idx].clone());
                }
            }
            (8, 8) => { // 레지쓰기 RSTORE — pop value, operands[0]=레지번호
                let val = self.pop("레지쓰기")?;
                let idx = operands.first()
                    .and_then(|v| v.as_int())
                    .unwrap_or(0) as usize;
                if idx < 9 {
                    self.registers[idx] = val;
                }
            }

            // 미구현 → NOP
            _ => {}
        }

        Ok(())
    }

    // ── 디버그/덤프 ──

    pub fn dump_stack(&self) {
        println!("╔══ 스택 (깊이: {}) ══╗", self.stack.len());
        for (i, v) in self.stack.iter().enumerate().rev() {
            println!("║ [{:3}] {:30} ({}) ║", i, format!("{}", v), v.type_name_kr());
        }
        println!("╚══════════════════════════════╝");
    }

    pub fn dump_registers(&self) {
        println!("╔══ 레지스터 (R0..R8) ══╗");
        for (i, v) in self.registers.iter().enumerate() {
            if !matches!(v, Value::Nil) {
                println!("║ R{}: {} ({}) ║", i, v, v.type_name_kr());
            }
        }
        println!("╚════════════════════════╝");
    }

    pub fn dump_all(&self) {
        self.dump_stack();
        self.dump_registers();
        self.heap.dump();
        println!("IP: {} | 사이클: {} | 종료: {}", self.ip, self.cycles, self.halted);
    }
}
