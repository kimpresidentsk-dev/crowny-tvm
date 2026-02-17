///! 한선어 어셈블러 — 한글/영문 니모닉 → TVM Instruction
///! 형식: 명령어 [피연산자...]
///!
///! 예시:
///!   넣어 42        ; PUSH 42
///!   넣어 7         ; PUSH 7
///!   더해           ; ADD
///!   보여줘         ; PRINT
///!   종료           ; HALT

use std::collections::HashMap;
use crate::opcode::{OpcodeAddr, build_opcodes, build_name_lookup};
use crate::trit::Trit;
use crate::value::Value;
use crate::vm::Instruction;

/// 피연산자 파싱
fn parse_operand(s: &str) -> Option<Value> {
    let s = s.trim();
    if s.is_empty() { return None; }

    // 문자열 리터럴
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        return Some(Value::Str(s[1..s.len()-1].to_string()));
    }

    // 특수 리터럴
    match s {
        "없다" | "없음" | "nil" | "NIL" => return Some(Value::Nil),
        "참" | "true" | "TRUE" => return Some(Value::Bool(true)),
        "거짓" | "false" | "FALSE" => return Some(Value::Bool(false)),
        "P" => return Some(Value::Trit(Trit::P)),
        "O" => return Some(Value::Trit(Trit::O)),
        "T" => return Some(Value::Trit(Trit::T)),
        _ => {}
    }

    // 실수 (소수점)
    if s.contains('.') {
        if let Ok(f) = s.parse::<f64>() {
            return Some(Value::Float(f));
        }
    }

    // 정수
    if let Ok(n) = s.parse::<i64>() {
        return Some(Value::Int(n));
    }

    // 기타 → 문자열
    Some(Value::Str(s.to_string()))
}

/// 어셈블리 소스 → 명령어 벡터
pub fn assemble(source: &str) -> Vec<Instruction> {
    let opcodes = build_opcodes();
    let name_lookup = build_name_lookup(&opcodes);

    let mut program = Vec::new();

    for (line_no, line) in source.lines().enumerate() {
        let line = line.trim();

        // 빈 줄, 주석 무시
        if line.is_empty() || line.starts_with(';') || line.starts_with("//") || line.starts_with('#') {
            continue;
        }

        // 인라인 주석 제거
        let line = if let Some(pos) = line.find(';') {
            &line[..pos]
        } else {
            line
        }.trim();

        if line.is_empty() { continue; }

        // 명령어 + 피연산자 분리
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        let cmd = parts[0];
        let arg_str = parts.get(1).map(|s| s.trim()).unwrap_or("");

        // 명령어 조회
        if let Some(addr) = name_lookup.get(cmd) {
            let operands: Vec<Value> = if arg_str.is_empty() {
                vec![]
            } else {
                // 쉼표 또는 공백으로 분리
                arg_str.split(',')
                    .flat_map(|s| s.split_whitespace())
                    .filter_map(|s| parse_operand(s))
                    .collect()
            };
            program.push(Instruction::from_addr(*addr, operands));
        } else {
            eprintln!("[어셈블러:{}행] 인식 불가: '{}'", line_no + 1, cmd);
        }
    }

    program
}

/// 디스어셈블: 명령어 벡터 → 읽기 가능한 문자열
pub fn disassemble(program: &[Instruction], opcodes: &HashMap<OpcodeAddr, crate::opcode::OpMeta>) -> String {
    let mut out = String::new();
    for (i, inst) in program.iter().enumerate() {
        let name = opcodes.get(&inst.addr).map(|m| m.name_kr).unwrap_or("???");
        let operand_str = if inst.operands.is_empty() {
            String::new()
        } else {
            format!(" {}", inst.operands.iter().map(|v| format!("{}", v)).collect::<Vec<_>>().join(", "))
        };
        out.push_str(&format!("{:04}: {} {}{}\n",
            i, inst.addr, name, operand_str));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_assembly() {
        let src = "넣어 10\n넣어 20\n더해\n보여줘\n종료";
        let prog = assemble(src);
        assert_eq!(prog.len(), 5);
    }

    #[test]
    fn test_english_mnemonics() {
        let src = "PUSH 42\nPUSH 8\nADD\nPRINT\nHALT";
        let prog = assemble(src);
        assert_eq!(prog.len(), 5);
    }
}
