///! ═══════════════════════════════════════════════════
///! 한선어 컴파일러 v0.1 — Crowny WebScript → TVM
///! ═══════════════════════════════════════════════════
///!
///! GPT Spec: "Crowny Script가 서버 로직까지 지배하는 플랫폼"
///!
///! 지원 문법:
///!   값 N              → 넣어 N
///!   변수 이름 = 값     → 넣어 값 / 저장해 슬롯
///!   이름              → 불러와 슬롯
///!   더 / 빼 / 곱 / 나눠 → 산술
///!   보여줘             → 출력
///!   만약 { } 아니면 { } → 3진 분기
///!   반복 N { }         → 루프
///!   함수 이름 { }      → 함수 정의
///!   이름()             → 함수 호출
///!   질문해 "프롬프트"   → LLM 호출
///!   끝                 → 종료

use std::collections::HashMap;
use crate::vm::Instruction;
use crate::opcode::OpcodeAddr;
use crate::value::Value;

// ─────────────────────────────────────────────
// 토큰
// ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    // 리터럴
    Int(i64),
    Float(f64),
    Str(String),
    Trit(i8),          // P(+1), O(0), T(-1)

    // 키워드
    Val,               // 값
    Var,               // 변수
    If,                // 만약
    Else,              // 아니면
    Neutral,           // 보류 (3진 분기 중간)
    Loop,              // 반복
    Func,              // 함수
    Return,            // 반환
    End,               // 끝
    Show,              // 보여줘
    Ask,               // 질문해

    // 연산
    Add,               // 더
    Sub,               // 빼
    Mul,               // 곱
    Div,               // 나눠
    Mod,               // 나머지
    Eq,                // 같다
    Neq,               // 다르다
    Gt,                // 크다
    Lt,                // 작다
    Not,               // 아니다
    And,               // 그리고

    // 구문
    Assign,            // =
    LBrace,            // {
    RBrace,            // }
    LParen,            // (
    RParen,            // )
    Comma,             // ,

    // 식별자
    Ident(String),

    // 끝
    Eof,
}

// ─────────────────────────────────────────────
// 렉서
// ─────────────────────────────────────────────

fn lex(source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars: Vec<char> = source.chars().collect();
    let mut pos = 0;

    while pos < chars.len() {
        let ch = chars[pos];

        // 공백
        if ch.is_whitespace() { pos += 1; continue; }

        // 주석
        if ch == ';' || (ch == '/' && pos + 1 < chars.len() && chars[pos+1] == '/') {
            while pos < chars.len() && chars[pos] != '\n' { pos += 1; }
            continue;
        }
        if ch == '#' {
            while pos < chars.len() && chars[pos] != '\n' { pos += 1; }
            continue;
        }

        // 기호
        match ch {
            '=' => { tokens.push(Token::Assign); pos += 1; continue; }
            '{' => { tokens.push(Token::LBrace); pos += 1; continue; }
            '}' => { tokens.push(Token::RBrace); pos += 1; continue; }
            '(' => { tokens.push(Token::LParen); pos += 1; continue; }
            ')' => { tokens.push(Token::RParen); pos += 1; continue; }
            ',' => { tokens.push(Token::Comma); pos += 1; continue; }
            _ => {}
        }

        // 문자열
        if ch == '"' || ch == '\'' {
            let quote = ch;
            pos += 1;
            let start = pos;
            while pos < chars.len() && chars[pos] != quote { pos += 1; }
            let s: String = chars[start..pos].iter().collect();
            tokens.push(Token::Str(s));
            if pos < chars.len() { pos += 1; }
            continue;
        }

        // 숫자
        if ch.is_ascii_digit() || (ch == '-' && pos + 1 < chars.len() && chars[pos+1].is_ascii_digit()) {
            let start = pos;
            if ch == '-' { pos += 1; }
            while pos < chars.len() && (chars[pos].is_ascii_digit() || chars[pos] == '.') {
                pos += 1;
            }
            let num_str: String = chars[start..pos].iter().collect();
            if num_str.contains('.') {
                if let Ok(f) = num_str.parse::<f64>() {
                    tokens.push(Token::Float(f));
                }
            } else if let Ok(n) = num_str.parse::<i64>() {
                tokens.push(Token::Int(n));
            }
            continue;
        }

        // 식별자/키워드
        if ch.is_alphabetic() || ch == '_' || ch >= '\u{AC00}' {
            let start = pos;
            while pos < chars.len() && (chars[pos].is_alphanumeric() || chars[pos] == '_' || chars[pos] >= '\u{AC00}') {
                pos += 1;
            }
            let word: String = chars[start..pos].iter().collect();
            let tok = match word.as_str() {
                "값" | "val" => Token::Val,
                "변수" | "var" | "let" => Token::Var,
                "만약" | "if" => Token::If,
                "아니면" | "else" => Token::Else,
                "보류" | "neutral" => Token::Neutral,
                "반복" | "loop" | "repeat" => Token::Loop,
                "함수" | "func" | "fn" => Token::Func,
                "반환" | "return" => Token::Return,
                "끝" | "end" | "종료" => Token::End,
                "보여줘" | "print" => Token::Show,
                "질문해" | "ask" | "llm" => Token::Ask,
                "더" | "더해" | "add" => Token::Add,
                "빼" | "sub" => Token::Sub,
                "곱" | "곱해" | "mul" => Token::Mul,
                "나눠" | "div" => Token::Div,
                "나머지" | "mod" => Token::Mod,
                "같다" | "eq" => Token::Eq,
                "다르다" | "neq" => Token::Neq,
                "크다" | "gt" => Token::Gt,
                "작다" | "lt" => Token::Lt,
                "아니다" | "not" => Token::Not,
                "그리고" | "and" => Token::And,
                "참" | "P" => Token::Trit(1),
                "모름" | "O" => Token::Trit(0),
                "거짓" | "T" => Token::Trit(-1),
                _ => Token::Ident(word),
            };
            tokens.push(tok);
            continue;
        }

        // 건너뛰기
        pos += 1;
    }

    tokens.push(Token::Eof);
    tokens
}

// ─────────────────────────────────────────────
// 컴파일러
// ─────────────────────────────────────────────

/// 컴파일 결과
pub struct CompileOutput {
    pub instructions: Vec<Instruction>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub variables: usize,
    pub functions: usize,
}

/// 한선어 컴파일러
pub struct HanseonCompiler {
    tokens: Vec<Token>,
    pos: usize,
    // 변수 테이블: 이름 → 슬롯 번호
    vars: HashMap<String, u32>,
    var_counter: u32,
    // 함수 테이블: 이름 → 명령어 시작 위치
    funcs: HashMap<String, usize>,
    output: Vec<Instruction>,
    warnings: Vec<String>,
    errors: Vec<String>,
}

impl HanseonCompiler {
    pub fn new(source: &str) -> Self {
        let tokens = lex(source);
        Self {
            tokens,
            pos: 0,
            vars: HashMap::new(),
            var_counter: 0,
            funcs: HashMap::new(),
            output: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// 컴파일 실행
    pub fn compile(mut self) -> CompileOutput {
        while self.peek() != &Token::Eof {
            self.compile_statement();
        }

        // 마지막에 HALT 보장
        if self.output.last().map(|i| i.addr != OpcodeAddr::new(0,2,7)).unwrap_or(true) {
            self.emit(OpcodeAddr::new(0, 2, 7), vec![]); // 종료
        }

        let var_count = self.vars.len();
        let func_count = self.funcs.len();
        CompileOutput {
            instructions: self.output,
            warnings: self.warnings,
            errors: self.errors,
            variables: var_count,
            functions: func_count,
        }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> bool {
        if self.peek() == expected {
            self.advance();
            true
        } else {
            self.errors.push(format!("예상: {:?}, 실제: {:?}", expected, self.peek()));
            false
        }
    }

    fn emit(&mut self, addr: OpcodeAddr, operands: Vec<Value>) {
        self.output.push(Instruction::from_addr(addr, operands));
    }

    // ── 문장 컴파일 ──

    fn compile_statement(&mut self) {
        match self.peek().clone() {
            Token::Val => self.compile_val(),
            Token::Var => self.compile_var(),
            Token::If => self.compile_if(),
            Token::Loop => self.compile_loop(),
            Token::Func => self.compile_func(),
            Token::Return => self.compile_return(),
            Token::End => { self.advance(); self.emit(OpcodeAddr::new(0,2,7), vec![]); }
            Token::Show => { self.advance(); self.emit(OpcodeAddr::new(0,3,5), vec![]); }
            Token::Ask => self.compile_ask(),

            // 산술 (후위 표기)
            Token::Add => { self.advance(); self.emit(OpcodeAddr::new(0,1,0), vec![]); }
            Token::Sub => { self.advance(); self.emit(OpcodeAddr::new(0,1,1), vec![]); }
            Token::Mul => { self.advance(); self.emit(OpcodeAddr::new(0,1,2), vec![]); }
            Token::Div => { self.advance(); self.emit(OpcodeAddr::new(0,1,3), vec![]); }
            Token::Mod => { self.advance(); self.emit(OpcodeAddr::new(0,1,4), vec![]); }

            // 비교
            Token::Eq => { self.advance(); self.emit(OpcodeAddr::new(0,0,3), vec![]); }
            Token::Neq => { self.advance(); self.emit(OpcodeAddr::new(0,0,4), vec![]); }
            Token::Gt => { self.advance(); self.emit(OpcodeAddr::new(0,0,5), vec![]); }
            Token::Lt => { self.advance(); self.emit(OpcodeAddr::new(0,0,6), vec![]); }
            Token::Not => { self.advance(); self.emit(OpcodeAddr::new(0,0,7), vec![]); }
            Token::And => { self.advance(); self.emit(OpcodeAddr::new(0,0,8), vec![]); }

            // 리터럴
            Token::Int(n) => { self.advance(); self.emit(OpcodeAddr::new(0,3,0), vec![Value::Int(n)]); }
            Token::Float(f) => { self.advance(); self.emit(OpcodeAddr::new(0,3,0), vec![Value::Float(f)]); }
            Token::Str(s) => { self.advance(); self.emit(OpcodeAddr::new(0,3,0), vec![Value::Str(s)]); }
            Token::Trit(t) => {
                self.advance();
                match t {
                    1 => self.emit(OpcodeAddr::new(0,0,0), vec![]),  // 참
                    -1 => self.emit(OpcodeAddr::new(0,0,1), vec![]), // 거짓
                    _ => self.emit(OpcodeAddr::new(0,0,2), vec![]),  // 모름
                }
            }

            // 식별자: 변수 로드 또는 함수 호출
            Token::Ident(name) => {
                self.advance();
                if self.peek() == &Token::LParen {
                    // 함수 호출
                    self.advance(); // (
                    // 인자는 미구현 (v0.1)
                    if self.peek() != &Token::RParen {
                        self.advance(); // skip arg
                    }
                    self.expect(&Token::RParen);
                    if let Some(&addr) = self.funcs.get(&name) {
                        self.emit(OpcodeAddr::new(0,2,2), vec![Value::Int(addr as i64)]);
                    } else {
                        self.errors.push(format!("정의되지 않은 함수: {}", name));
                    }
                } else if let Some(&slot) = self.vars.get(&name) {
                    // 변수 로드
                    self.emit(OpcodeAddr::new(0,3,8), vec![Value::Int(slot as i64)]);
                } else {
                    self.errors.push(format!("정의되지 않은 변수: {}", name));
                }
            }

            Token::RBrace => { self.advance(); } // 블록 닫기
            Token::Eof => {}
            _ => {
                let tok = self.advance();
                self.warnings.push(format!("무시된 토큰: {:?}", tok));
            }
        }
    }

    // ── 값 N ──
    fn compile_val(&mut self) {
        self.advance(); // '값'
        match self.advance() {
            Token::Int(n) => self.emit(OpcodeAddr::new(0,3,0), vec![Value::Int(n)]),
            Token::Float(f) => self.emit(OpcodeAddr::new(0,3,0), vec![Value::Float(f)]),
            Token::Str(s) => self.emit(OpcodeAddr::new(0,3,0), vec![Value::Str(s)]),
            _ => self.errors.push("값 뒤에 리터럴 필요".into()),
        }
    }

    // ── 변수 이름 = 값 ──
    fn compile_var(&mut self) {
        self.advance(); // '변수'
        if let Token::Ident(name) = self.advance() {
            self.expect(&Token::Assign);
            // 값 컴파일 (스택에 push)
            self.compile_statement();
            // 변수 슬롯 할당
            let slot = if let Some(&s) = self.vars.get(&name) {
                s
            } else {
                let s = self.var_counter;
                self.vars.insert(name, s);
                self.var_counter += 1;
                s
            };
            self.emit(OpcodeAddr::new(0,3,7), vec![Value::Int(slot as i64)]); // 저장해
        } else {
            self.errors.push("변수 뒤에 이름 필요".into());
        }
    }

    // ── 만약 { P블록 } 보류 { O블록 } 아니면 { T블록 } ──
    fn compile_if(&mut self) {
        self.advance(); // '만약'

        // P 블록 (+1)
        if self.peek() == &Token::LBrace {
            self.advance();
            while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
                self.compile_statement();
            }
            self.expect(&Token::RBrace);
        }

        // 보류 블록 (0) - 선택적
        if self.peek() == &Token::Neutral {
            self.advance();
            if self.peek() == &Token::LBrace {
                self.advance();
                while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
                    self.compile_statement();
                }
                self.expect(&Token::RBrace);
            }
        }

        // 아니면 블록 (-1) - 선택적
        if self.peek() == &Token::Else {
            self.advance();
            if self.peek() == &Token::LBrace {
                self.advance();
                while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
                    self.compile_statement();
                }
                self.expect(&Token::RBrace);
            }
        }
    }

    // ── 반복 N { } ──
    fn compile_loop(&mut self) {
        self.advance(); // '반복'
        let count = match self.advance() {
            Token::Int(n) => n,
            _ => { self.errors.push("반복 뒤에 횟수 필요".into()); 1 }
        };

        // 카운터 push
        self.emit(OpcodeAddr::new(0,3,0), vec![Value::Int(count)]);

        let loop_start = self.output.len();

        if self.peek() == &Token::LBrace {
            self.advance();
            while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
                self.compile_statement();
            }
            self.expect(&Token::RBrace);
        }

        // 반복 명령
        self.emit(OpcodeAddr::new(0,2,4), vec![Value::Int(loop_start as i64)]);
    }

    // ── 함수 이름 { } ──
    fn compile_func(&mut self) {
        self.advance(); // '함수'
        if let Token::Ident(name) = self.advance() {
            let func_start = self.output.len();
            self.funcs.insert(name, func_start);

            // 함수 시작 마커
            self.emit(OpcodeAddr::new(0,4,0), vec![]);

            if self.peek() == &Token::LBrace {
                self.advance();
                while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
                    self.compile_statement();
                }
                self.expect(&Token::RBrace);
            }

            // 반환
            self.emit(OpcodeAddr::new(0,2,3), vec![]);
        } else {
            self.errors.push("함수 뒤에 이름 필요".into());
        }
    }

    // ── 반환 ──
    fn compile_return(&mut self) {
        self.advance();
        self.emit(OpcodeAddr::new(0,2,3), vec![]);
    }

    // ── 질문해 "프롬프트" ──
    fn compile_ask(&mut self) {
        self.advance(); // '질문해'
        match self.advance() {
            Token::Str(prompt) => {
                self.emit(OpcodeAddr::new(0,3,0), vec![Value::Str(prompt)]); // 프롬프트 push
                self.emit(OpcodeAddr::new(1,0,0), vec![]); // LLM_ASK (섹터1)
            }
            _ => {
                self.errors.push("질문해 뒤에 문자열 필요".into());
            }
        }
    }
}

/// 한선어 소스 → TVM 프로그램 (원스톱)
pub fn compile(source: &str) -> CompileOutput {
    HanseonCompiler::new(source).compile()
}

/// 한선어 → TVM → WASM (전체 파이프라인)
pub fn compile_to_wasm(source: &str) -> Vec<u8> {
    let output = compile(source);
    crate::compiler::compile_to_wasm(&output.instructions, "crowny")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_calc() {
        let out = compile("값 5\n값 3\n더\n보여줘\n끝");
        assert!(out.errors.is_empty(), "에러: {:?}", out.errors);
        assert!(out.instructions.len() >= 4);
    }

    #[test]
    fn test_variables() {
        let out = compile("변수 x = 10\n변수 y = 20\nx\ny\n더\n보여줘\n끝");
        assert!(out.errors.is_empty(), "에러: {:?}", out.errors);
        assert_eq!(out.variables, 2);
    }

    #[test]
    fn test_function() {
        let out = compile("함수 인사 {\n보여줘\n}\n값 42\n보여줘\n끝");
        assert!(out.errors.is_empty(), "에러: {:?}", out.errors);
        assert_eq!(out.functions, 1);
    }

    #[test]
    fn test_trit_logic() {
        let out = compile("참\n모름\n그리고\n끝");
        assert!(out.errors.is_empty(), "에러: {:?}", out.errors);
    }

    #[test]
    fn test_loop() {
        let out = compile("반복 3 {\n값 1\n보여줘\n}\n끝");
        assert!(out.errors.is_empty(), "에러: {:?}", out.errors);
    }

    #[test]
    fn test_llm_call() {
        let out = compile("질문해 \"오늘 날씨?\"\n보여줘\n끝");
        assert!(out.errors.is_empty(), "에러: {:?}", out.errors);
        // 섹터 1 명령이 포함되었는지 확인
        let has_llm = out.instructions.iter().any(|i| i.addr.sector == 1);
        assert!(has_llm, "LLM opcode 없음");
    }

    #[test]
    fn test_english_syntax() {
        let out = compile("val 10\nval 20\nadd\nprint\nend");
        assert!(out.errors.is_empty(), "에러: {:?}", out.errors);
    }

    #[test]
    fn test_compile_to_wasm() {
        let wasm = compile_to_wasm("값 42\n끝");
        assert_eq!(&wasm[0..4], b"\0asm");
    }
}
