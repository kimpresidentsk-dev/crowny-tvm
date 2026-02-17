///! ═══════════════════════════════════════════════════
///! WASM Binary Generator — .wasm 파일 직접 생성
///! ═══════════════════════════════════════════════════
///!
///! IR → WASM Binary (.wasm)
///!
///! WASM MVP 바이너리 포맷 준수:
///!   Magic:   \0asm
///!   Version: 1
///!   Sections: Type, Import, Function, Memory, Export, Code
///!
///! GPT Spec §3: 변환기 구조
///! GPT Spec §7: 실행 흐름

use crate::ir::*;

// ─────────────────────────────────────────────
// WASM 상수
// ─────────────────────────────────────────────

// Section IDs
const SEC_TYPE: u8 = 1;
const SEC_IMPORT: u8 = 2;
const SEC_FUNCTION: u8 = 3;
const SEC_MEMORY: u8 = 5;
const SEC_GLOBAL: u8 = 6;
const SEC_EXPORT: u8 = 7;
const SEC_START: u8 = 8;
const SEC_CODE: u8 = 10;

// Value types
const WASM_I32: u8 = 0x7F;
const WASM_I64: u8 = 0x7E;
const WASM_F64: u8 = 0x7C;

// Block types
const WASM_VOID: u8 = 0x40;

// Export kinds
const EXPORT_FUNC: u8 = 0x00;
const EXPORT_MEMORY: u8 = 0x02;

// ─────────────────────────────────────────────
// LEB128 인코딩
// ─────────────────────────────────────────────

fn encode_u32_leb128(mut val: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        bytes.push(byte);
        if val == 0 { break; }
    }
    bytes
}

fn encode_i32_leb128(mut val: i32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut more = true;
    while more {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if (val == 0 && (byte & 0x40) == 0) || (val == -1 && (byte & 0x40) != 0) {
            more = false;
        } else {
            byte |= 0x80;
        }
        bytes.push(byte);
    }
    bytes
}

fn encode_i64_leb128(mut val: i64) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut more = true;
    while more {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if (val == 0 && (byte & 0x40) == 0) || (val == -1 && (byte & 0x40) != 0) {
            more = false;
        } else {
            byte |= 0x80;
        }
        bytes.push(byte);
    }
    bytes
}

fn encode_string(s: &str) -> Vec<u8> {
    let mut bytes = encode_u32_leb128(s.len() as u32);
    bytes.extend_from_slice(s.as_bytes());
    bytes
}

fn ir_type_to_wasm(t: &IrType) -> u8 {
    match t {
        IrType::I32 => WASM_I32,
        IrType::I64 => WASM_I64,
        IrType::F64 => WASM_F64,
    }
}

// ─────────────────────────────────────────────
// WasmModule Builder
// ─────────────────────────────────────────────

/// WASM 모듈 빌더
pub struct WasmBuilder {
    /// 생성된 바이너리
    bytes: Vec<u8>,
}

impl WasmBuilder {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// IR 모듈 → WASM 바이너리 생성
    pub fn build(ir: &IrModule) -> Vec<u8> {
        let mut builder = Self::new();

        // Magic + Version
        builder.emit_header();

        // Type Section — 함수 시그니처
        builder.emit_type_section(ir);

        // Import Section
        if !ir.imports.is_empty() {
            builder.emit_import_section(ir);
        }

        // Function Section — 함수 인덱스 → 타입 인덱스 매핑
        builder.emit_function_section(ir);

        // Memory Section
        builder.emit_memory_section(ir);

        // Global Section
        if !ir.globals.is_empty() {
            builder.emit_global_section(ir);
        }

        // Export Section
        builder.emit_export_section(ir);

        // Start Section
        if let Some(start) = ir.start_fn {
            builder.emit_start_section(start);
        }

        // Code Section — 함수 본문
        builder.emit_code_section(ir);

        builder.bytes
    }

    // ── Header ──

    fn emit_header(&mut self) {
        self.bytes.extend_from_slice(b"\0asm");  // Magic
        self.bytes.extend_from_slice(&[1, 0, 0, 0]);  // Version 1
    }

    // ── Section Helper ──

    fn emit_section(&mut self, id: u8, content: &[u8]) {
        self.bytes.push(id);
        self.bytes.extend_from_slice(&encode_u32_leb128(content.len() as u32));
        self.bytes.extend_from_slice(content);
    }

    // ── Type Section (1) ──

    fn emit_type_section(&mut self, ir: &IrModule) {
        let mut content = Vec::new();
        let type_count = ir.imports.len() + ir.functions.len();
        content.extend_from_slice(&encode_u32_leb128(type_count as u32));

        // Import function types
        for imp in &ir.imports {
            content.push(0x60); // functype
            content.extend_from_slice(&encode_u32_leb128(imp.params.len() as u32));
            for p in &imp.params {
                content.push(ir_type_to_wasm(p));
            }
            content.extend_from_slice(&encode_u32_leb128(imp.results.len() as u32));
            for r in &imp.results {
                content.push(ir_type_to_wasm(r));
            }
        }

        // Local function types
        for func in &ir.functions {
            content.push(0x60); // functype
            content.extend_from_slice(&encode_u32_leb128(func.params.len() as u32));
            for p in &func.params {
                content.push(ir_type_to_wasm(p));
            }
            content.extend_from_slice(&encode_u32_leb128(func.results.len() as u32));
            for r in &func.results {
                content.push(ir_type_to_wasm(r));
            }
        }

        self.emit_section(SEC_TYPE, &content);
    }

    // ── Import Section (2) ──

    fn emit_import_section(&mut self, ir: &IrModule) {
        let mut content = Vec::new();
        content.extend_from_slice(&encode_u32_leb128(ir.imports.len() as u32));

        for (i, imp) in ir.imports.iter().enumerate() {
            content.extend_from_slice(&encode_string(&imp.module));
            content.extend_from_slice(&encode_string(&imp.name));
            content.push(0x00); // import kind = function
            content.extend_from_slice(&encode_u32_leb128(i as u32)); // type index
        }

        self.emit_section(SEC_IMPORT, &content);
    }

    // ── Function Section (3) ──

    fn emit_function_section(&mut self, ir: &IrModule) {
        let mut content = Vec::new();
        content.extend_from_slice(&encode_u32_leb128(ir.functions.len() as u32));

        let import_count = ir.imports.len() as u32;
        for i in 0..ir.functions.len() {
            let type_idx = import_count + i as u32;
            content.extend_from_slice(&encode_u32_leb128(type_idx));
        }

        self.emit_section(SEC_FUNCTION, &content);
    }

    // ── Memory Section (5) ──

    fn emit_memory_section(&mut self, ir: &IrModule) {
        let mut content = Vec::new();
        content.push(1); // 1 memory
        content.push(0x00); // limits: min only
        content.extend_from_slice(&encode_u32_leb128(ir.memory_pages));
        self.emit_section(SEC_MEMORY, &content);
    }

    // ── Global Section (6) ──

    fn emit_global_section(&mut self, ir: &IrModule) {
        let mut content = Vec::new();
        content.extend_from_slice(&encode_u32_leb128(ir.globals.len() as u32));

        for g in &ir.globals {
            content.push(ir_type_to_wasm(&g.typ));
            content.push(if g.mutable { 0x01 } else { 0x00 });
            // init expr
            match g.typ {
                IrType::I64 => {
                    content.push(0x42); // i64.const
                    content.extend_from_slice(&encode_i64_leb128(g.init_value));
                }
                IrType::I32 => {
                    content.push(0x41); // i32.const
                    content.extend_from_slice(&encode_i32_leb128(g.init_value as i32));
                }
                _ => {
                    content.push(0x42);
                    content.extend_from_slice(&encode_i64_leb128(0));
                }
            }
            content.push(0x0B); // end
        }

        self.emit_section(SEC_GLOBAL, &content);
    }

    // ── Export Section (7) ──

    fn emit_export_section(&mut self, ir: &IrModule) {
        let mut exports: Vec<(&str, u8, u32)> = Vec::new();

        // Memory export
        exports.push(("memory", EXPORT_MEMORY, 0));

        // Function exports
        let import_count = ir.imports.len() as u32;
        for (i, func) in ir.functions.iter().enumerate() {
            if func.is_export {
                exports.push((&func.name, EXPORT_FUNC, import_count + i as u32));
            }
        }

        let mut content = Vec::new();
        content.extend_from_slice(&encode_u32_leb128(exports.len() as u32));

        for (name, kind, idx) in &exports {
            content.extend_from_slice(&encode_string(name));
            content.push(*kind);
            content.extend_from_slice(&encode_u32_leb128(*idx));
        }

        self.emit_section(SEC_EXPORT, &content);
    }

    // ── Start Section (8) ──

    fn emit_start_section(&mut self, func_idx: u32) {
        let content = encode_u32_leb128(func_idx);
        self.emit_section(SEC_START, &content);
    }

    // ── Code Section (10) ──

    fn emit_code_section(&mut self, ir: &IrModule) {
        let mut content = Vec::new();
        content.extend_from_slice(&encode_u32_leb128(ir.functions.len() as u32));

        let import_count = ir.imports.len() as u32;

        for func in &ir.functions {
            let body = Self::emit_function_body(func, import_count);
            content.extend_from_slice(&encode_u32_leb128(body.len() as u32));
            content.extend_from_slice(&body);
        }

        self.emit_section(SEC_CODE, &content);
    }

    /// 함수 본문 바이트코드 생성
    fn emit_function_body(func: &IrFunction, import_count: u32) -> Vec<u8> {
        let mut body = Vec::new();

        // Locals (params 제외, 추가 locals만)
        if func.locals.is_empty() {
            body.push(0); // 0 local declarations
        } else {
            // 같은 타입끼리 묶기
            let mut groups: Vec<(u32, u8)> = Vec::new();
            for local in &func.locals {
                let wt = ir_type_to_wasm(local);
                if let Some(last) = groups.last_mut() {
                    if last.1 == wt {
                        last.0 += 1;
                        continue;
                    }
                }
                groups.push((1, wt));
            }
            body.extend_from_slice(&encode_u32_leb128(groups.len() as u32));
            for (count, typ) in &groups {
                body.extend_from_slice(&encode_u32_leb128(*count));
                body.push(*typ);
            }
        }

        // Instructions
        for op in &func.body {
            Self::emit_ir_op(&mut body, op, import_count);
        }

        // End
        body.push(0x0B);

        body
    }

    /// IR 명령 → WASM opcode
    fn emit_ir_op(out: &mut Vec<u8>, op: &IrOp, import_count: u32) {
        match op {
            // ── 스택 ──
            IrOp::Const(v) => {
                out.push(0x42); // i64.const
                out.extend_from_slice(&encode_i64_leb128(*v));
            }
            IrOp::ConstF64(v) => {
                out.push(0x44); // f64.const
                out.extend_from_slice(&v.to_le_bytes());
            }
            IrOp::ConstTrit(t) => {
                // Trit → i64 (-1, 0, +1)
                out.push(0x42);
                out.extend_from_slice(&encode_i64_leb128(*t as i64));
            }
            IrOp::Drop => { out.push(0x1A); }
            IrOp::Dup => {
                // WASM에 dup 없음 → local.tee 패턴 사용 불가 (스택 전용)
                // TVM에서 dup은 컴파일러가 local로 변환해야 함
                // 여기서는 nop (컴파일러 단계에서 처리)
                out.push(0x01); // nop placeholder
            }
            IrOp::Swap => {
                // WASM에 swap 없음 → local 사용 (컴파일러 단계 처리)
                out.push(0x01);
            }

            // ── 산술 (i64) ──
            IrOp::Add => { out.push(0x7C); } // i64.add
            IrOp::Sub => { out.push(0x7D); } // i64.sub
            IrOp::Mul => { out.push(0x7E); } // i64.mul
            IrOp::Div => { out.push(0x7F); } // i64.div_s
            IrOp::Rem => { out.push(0x81); } // i64.rem_s
            IrOp::Neg => {
                // -x = 0 - x
                out.push(0x42); // i64.const 0
                out.push(0x00);
                // swap (value is already on stack)
                // actually: emit 0 first, then the value needs to be there
                // This requires reordering — handle in compiler
                out.push(0x7D); // i64.sub
            }
            IrOp::Abs => {
                // abs: if x < 0 then -x else x
                // Complex — handled as inline
                out.push(0x01); // nop (compiler handles)
            }
            IrOp::Min | IrOp::Max => {
                out.push(0x01); // nop (compiler generates comparison sequence)
            }

            // ── 비교 ──
            IrOp::Eq => { out.push(0x51); } // i64.eq
            IrOp::Ne => { out.push(0x52); } // i64.ne
            IrOp::Lt => { out.push(0x53); } // i64.lt_s
            IrOp::Gt => { out.push(0x55); } // i64.gt_s
            IrOp::Le => { out.push(0x57); } // i64.le_s
            IrOp::Ge => { out.push(0x59); } // i64.ge_s
            IrOp::Eqz => { out.push(0x50); } // i64.eqz

            // ── 제어흐름 ──
            IrOp::Block(_) => {
                out.push(0x02); // block
                out.push(WASM_VOID);
            }
            IrOp::Loop(_) => {
                out.push(0x03); // loop
                out.push(WASM_VOID);
            }
            IrOp::Br(label) => {
                out.push(0x0C); // br
                out.extend_from_slice(&encode_u32_leb128(*label));
            }
            IrOp::BrIf(label) => {
                out.push(0x0D); // br_if
                out.extend_from_slice(&encode_u32_leb128(*label));
            }
            IrOp::Call(idx) => {
                out.push(0x10); // call
                out.extend_from_slice(&encode_u32_leb128(*idx));
            }
            IrOp::Return => { out.push(0x0F); }
            IrOp::End => { out.push(0x0B); }
            IrOp::Halt => {
                out.push(0x00); // unreachable
            }

            // ── 메모리 ──
            IrOp::MemLoad(offset) => {
                out.push(0x29); // i64.load
                out.push(0x03); // alignment (8 bytes)
                out.extend_from_slice(&encode_u32_leb128(*offset));
            }
            IrOp::MemStore(offset) => {
                out.push(0x37); // i64.store
                out.push(0x03); // alignment
                out.extend_from_slice(&encode_u32_leb128(*offset));
            }
            IrOp::MemGrow => {
                out.push(0x40); // memory.grow
                out.push(0x00); // memory index
            }

            // ── 로컬/전역 ──
            IrOp::LocalGet(idx) => {
                out.push(0x20);
                out.extend_from_slice(&encode_u32_leb128(*idx));
            }
            IrOp::LocalSet(idx) => {
                out.push(0x21);
                out.extend_from_slice(&encode_u32_leb128(*idx));
            }
            IrOp::GlobalGet(idx) => {
                out.push(0x23);
                out.extend_from_slice(&encode_u32_leb128(*idx));
            }
            IrOp::GlobalSet(idx) => {
                out.push(0x24);
                out.extend_from_slice(&encode_u32_leb128(*idx));
            }

            // ── 타입 변환 ──
            IrOp::I64ExtendI32 => { out.push(0xAC); } // i64.extend_i32_s
            IrOp::F64ConvertI64 => { out.push(0xB9); } // f64.convert_i64_s
            IrOp::I64TruncF64 => { out.push(0xB0); }   // i64.trunc_f64_s

            // ── IO ──
            IrOp::CallImport(idx) => {
                out.push(0x10); // call
                out.extend_from_slice(&encode_u32_leb128(*idx));
            }
            IrOp::Print => {
                out.push(0x10); // call $print (import index 0)
                out.push(0x00);
            }
            IrOp::Input => {
                out.push(0x10); // call $input (import index 1)
                out.push(0x01);
            }

            // ── Trit 전용 ──
            IrOp::TritClamp => {
                // clamp to -1,0,+1:
                // val = max(-1, min(1, val))
                // i64.const 1 / i64.lt_s / select pattern
                // Simplified: use Crowny runtime helper
                out.push(0x01); // nop (runtime handles)
            }
            IrOp::TritAnd => {
                // min(a, b) — 3진 AND
                // (a < b) ? a : b
                out.push(0x01); // compiler generates comparison
            }
            IrOp::TritOr => {
                // max(a, b)
                out.push(0x01);
            }
            IrOp::TritNot => {
                // 0 - val
                out.push(0x42); out.push(0x00); // i64.const 0
                // need swap here
                out.push(0x7D); // i64.sub
            }
            IrOp::TritBranch => {
                // 3진 분기: 컴파일러가 if-chain으로 변환
                out.push(0x01);
            }

            // ── NOP ──
            IrOp::Nop => { out.push(0x01); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leb128() {
        assert_eq!(encode_u32_leb128(0), vec![0x00]);
        assert_eq!(encode_u32_leb128(1), vec![0x01]);
        assert_eq!(encode_u32_leb128(127), vec![0x7F]);
        assert_eq!(encode_u32_leb128(128), vec![0x80, 0x01]);
    }

    #[test]
    fn test_minimal_wasm() {
        let mut module = IrModule::new("test");

        // 간단한 함수: () → i64, returns 42
        let mut func = IrFunction::new("answer");
        func.results.push(IrType::I64);
        func.body.push(IrOp::Const(42));
        func.is_export = true;
        module.add_function(func);

        let wasm = WasmBuilder::build(&module);

        // Magic 확인
        assert_eq!(&wasm[0..4], b"\0asm");
        // Version 확인
        assert_eq!(&wasm[4..8], &[1, 0, 0, 0]);
        // 크기 확인
        assert!(wasm.len() > 8, "WASM too short: {} bytes", wasm.len());

        println!("WASM 크기: {} bytes", wasm.len());
        println!("처음 32 bytes: {:02X?}", &wasm[..32.min(wasm.len())]);
    }

    #[test]
    fn test_add_function() {
        let mut module = IrModule::new("calc");

        // import: print(i64)
        module.imports.push(IrImport {
            module: "env".into(),
            name: "print".into(),
            params: vec![IrType::I64],
            results: vec![],
        });

        // 함수: add() → i64 = 5 + 3
        let mut func = IrFunction::new("add");
        func.results.push(IrType::I64);
        func.body.push(IrOp::Const(5));
        func.body.push(IrOp::Const(3));
        func.body.push(IrOp::Add);
        func.is_export = true;
        module.add_function(func);

        let wasm = WasmBuilder::build(&module);
        assert!(&wasm[0..4] == b"\0asm");
        println!("add() WASM: {} bytes", wasm.len());
    }
}
