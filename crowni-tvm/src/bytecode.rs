///! ═══════════════════════════════════════════════════
///! 크라운 바이트코드 — .크라운 바이너리 직렬화
///! ═══════════════════════════════════════════════════
///!
///! TVM 프로그램을 .크라운 파일로 저장/로드.
///!
///! 파일 구조:
///! ┌──────────────────────────────────────────┐
///! │ Magic:    0xCB 0x33 0xCB 0x33  (4 bytes) │
///! │ Version:  0x01                 (1 byte)  │
///! │ Flags:    0x00                 (1 byte)  │
///! │ InstCount: u32 LE             (4 bytes)  │
///! │ ─── 명령어 블록 (N개) ─────────────────── │
///! │  Sector:  u8                   (1 byte)  │
///! │  Group:   u8                   (1 byte)  │
///! │  Command: u8                   (1 byte)  │
///! │  OpCount: u8                   (1 byte)  │
///! │  Operand[0..N]: tagged values            │
///! └──────────────────────────────────────────┘
///!
///! 피연산자 태그:
///!   0x00 = None
///!   0x01 = Int(i64)     → 8 bytes LE
///!   0x02 = Float(f64)   → 8 bytes LE
///!   0x03 = Bool(u8)     → 1 byte
///!   0x04 = Trit(i8)     → 1 byte
///!   0x05 = Str(len+data)→ u16 LE + UTF-8
///!   0x06 = Nil          → 0 bytes

use crate::vm::Instruction;
use crate::opcode::OpcodeAddr;
use crate::value::Value;
use crate::trit::Trit;

/// 매직 넘버: CB33 CB33 (Crowny Balanced 3-3)
const MAGIC: [u8; 4] = [0xCB, 0x33, 0xCB, 0x33];
const VERSION: u8 = 1;

// 태그 상수
const TAG_NONE: u8 = 0x00;
const TAG_INT: u8 = 0x01;
const TAG_FLOAT: u8 = 0x02;
const TAG_BOOL: u8 = 0x03;
const TAG_TRIT: u8 = 0x04;
const TAG_STR: u8 = 0x05;
const TAG_NIL: u8 = 0x06;

/// TVM 프로그램 → .크라운 바이트코드 직렬화
pub fn serialize(program: &[Instruction]) -> Vec<u8> {
    let mut bytes = Vec::new();

    // Header
    bytes.extend_from_slice(&MAGIC);
    bytes.push(VERSION);
    bytes.push(0x00); // flags

    // Instruction count
    let count = program.len() as u32;
    bytes.extend_from_slice(&count.to_le_bytes());

    // Instructions
    for inst in program {
        // Opcode address (3 bytes)
        bytes.push(inst.addr.sector);
        bytes.push(inst.addr.group);
        bytes.push(inst.addr.command);

        // Operand count
        bytes.push(inst.operands.len() as u8);

        // Operands
        for op in &inst.operands {
            serialize_value(&mut bytes, op);
        }
    }

    bytes
}

/// .크라운 바이트코드 → TVM 프로그램 역직렬화
pub fn deserialize(data: &[u8]) -> Result<Vec<Instruction>, String> {
    if data.len() < 10 {
        return Err("파일 너무 짧음".into());
    }

    // Magic check
    if &data[0..4] != &MAGIC {
        return Err("매직 넘버 불일치 (크라운 파일 아님)".into());
    }

    // Version
    let version = data[4];
    if version != VERSION {
        return Err(format!("지원하지 않는 버전: {}", version));
    }

    // Instruction count
    let count = u32::from_le_bytes([data[6], data[7], data[8], data[9]]) as usize;

    let mut pos = 10;
    let mut program = Vec::with_capacity(count);

    for _ in 0..count {
        if pos + 4 > data.len() {
            return Err("명령어 데이터 부족".into());
        }

        let sector = data[pos];
        let group = data[pos + 1];
        let command = data[pos + 2];
        let op_count = data[pos + 3] as usize;
        pos += 4;

        let addr = OpcodeAddr::new(sector, group, command);
        let mut operands = Vec::with_capacity(op_count);

        for _ in 0..op_count {
            let (val, consumed) = deserialize_value(&data[pos..])?;
            operands.push(val);
            pos += consumed;
        }

        program.push(Instruction::from_addr(addr, operands));
    }

    Ok(program)
}

fn serialize_value(bytes: &mut Vec<u8>, val: &Value) {
    match val {
        Value::Int(n) => {
            bytes.push(TAG_INT);
            bytes.extend_from_slice(&n.to_le_bytes());
        }
        Value::Float(f) => {
            bytes.push(TAG_FLOAT);
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        Value::Bool(b) => {
            bytes.push(TAG_BOOL);
            bytes.push(if *b { 1 } else { 0 });
        }
        Value::Trit(t) => {
            bytes.push(TAG_TRIT);
            bytes.push(t.to_i8() as u8);
        }
        Value::Str(s) => {
            bytes.push(TAG_STR);
            let len = s.len().min(65535) as u16;
            bytes.extend_from_slice(&len.to_le_bytes());
            bytes.extend_from_slice(&s.as_bytes()[..len as usize]);
        }
        Value::Nil => {
            bytes.push(TAG_NIL);
        }
        _ => {
            bytes.push(TAG_NONE);
        }
    }
}

fn deserialize_value(data: &[u8]) -> Result<(Value, usize), String> {
    if data.is_empty() {
        return Err("값 데이터 부족".into());
    }

    let tag = data[0];
    match tag {
        TAG_INT => {
            if data.len() < 9 { return Err("Int 데이터 부족".into()); }
            let n = i64::from_le_bytes([
                data[1], data[2], data[3], data[4],
                data[5], data[6], data[7], data[8],
            ]);
            Ok((Value::Int(n), 9))
        }
        TAG_FLOAT => {
            if data.len() < 9 { return Err("Float 데이터 부족".into()); }
            let f = f64::from_le_bytes([
                data[1], data[2], data[3], data[4],
                data[5], data[6], data[7], data[8],
            ]);
            Ok((Value::Float(f), 9))
        }
        TAG_BOOL => {
            if data.len() < 2 { return Err("Bool 데이터 부족".into()); }
            Ok((Value::Bool(data[1] != 0), 2))
        }
        TAG_TRIT => {
            if data.len() < 2 { return Err("Trit 데이터 부족".into()); }
            let t = Trit::from_i8(data[1] as i8);
            Ok((Value::Trit(t), 2))
        }
        TAG_STR => {
            if data.len() < 3 { return Err("Str 길이 부족".into()); }
            let len = u16::from_le_bytes([data[1], data[2]]) as usize;
            if data.len() < 3 + len { return Err("Str 데이터 부족".into()); }
            let s = String::from_utf8_lossy(&data[3..3+len]).to_string();
            Ok((Value::Str(s), 3 + len))
        }
        TAG_NIL => Ok((Value::Nil, 1)),
        TAG_NONE => Ok((Value::Nil, 1)),
        _ => Err(format!("알 수 없는 태그: 0x{:02X}", tag)),
    }
}

/// 파일 정보
pub struct BytecodeInfo {
    pub version: u8,
    pub instruction_count: usize,
    pub byte_size: usize,
    pub avg_bytes_per_inst: f32,
}

/// 바이트코드 정보 분석
pub fn analyze(data: &[u8]) -> Result<BytecodeInfo, String> {
    if data.len() < 10 || &data[0..4] != &MAGIC {
        return Err("유효하지 않은 크라운 파일".into());
    }
    let count = u32::from_le_bytes([data[6], data[7], data[8], data[9]]) as usize;
    Ok(BytecodeInfo {
        version: data[4],
        instruction_count: count,
        byte_size: data.len(),
        avg_bytes_per_inst: if count > 0 { data.len() as f32 / count as f32 } else { 0.0 },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembler::assemble;

    #[test]
    fn test_roundtrip() {
        let source = "넣어 42\n넣어 7\n더해\n보여줘\n종료";
        let program = assemble(source);
        let bytes = serialize(&program);

        // Magic 확인
        assert_eq!(&bytes[0..4], &MAGIC);
        assert_eq!(bytes[4], VERSION);

        // 역직렬화
        let restored = deserialize(&bytes).unwrap();
        assert_eq!(restored.len(), program.len());

        for (orig, rest) in program.iter().zip(restored.iter()) {
            assert_eq!(orig.addr, rest.addr);
            assert_eq!(orig.operands.len(), rest.operands.len());
        }
    }

    #[test]
    fn test_string_operand() {
        let source = "넣어 \"안녕하세요\"\n보여줘\n종료";
        let program = assemble(source);
        let bytes = serialize(&program);
        let restored = deserialize(&bytes).unwrap();
        assert_eq!(restored.len(), program.len());
    }

    #[test]
    fn test_analyze() {
        let source = "넣어 1\n넣어 2\n더해\n종료";
        let program = assemble(source);
        let bytes = serialize(&program);
        let info = analyze(&bytes).unwrap();
        assert_eq!(info.instruction_count, 4);
        assert_eq!(info.version, 1);
    }
}
