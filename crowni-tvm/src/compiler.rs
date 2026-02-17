///! ═══════════════════════════════════════════════════
///! TVM → WASM Compiler — 균형3진 → 웹 변환기
///! ═══════════════════════════════════════════════════
///!
///! GPT Spec §3: 변환기 구조
///!   TVM Bytecode → IR → WASM Module → .wasm binary
///!
///! GPT Spec §7: 실행 흐름
///!   넣기 5 → i64.const 5
///!   넣기 3 → i64.const 3
///!   더     → i64.add
///!   출력   → call $print
///!   종료   → return
///!
///! 균형3진 의미는 유지, 실행은 WASM(2진).

use crate::vm::Instruction;
use crate::ir::*;
use crate::wasm_gen::WasmBuilder;
use crate::value::Value;

// ─────────────────────────────────────────────
// TVM → IR 변환기
// ─────────────────────────────────────────────

/// TVM 프로그램을 IR 모듈로 변환
pub fn tvm_to_ir(program: &[Instruction], module_name: &str) -> IrModule {
    let mut module = IrModule::new(module_name);

    // ── 표준 import 함수 등록 ──
    // [0] env.print(i64) — 출력
    module.imports.push(IrImport {
        module: "env".into(),
        name: "print".into(),
        params: vec![IrType::I64],
        results: vec![],
    });

    // [1] env.print_f64(f64) — 실수 출력
    module.imports.push(IrImport {
        module: "env".into(),
        name: "print_f64".into(),
        params: vec![IrType::F64],
        results: vec![],
    });

    // [2] env.input() → i64 — 입력
    module.imports.push(IrImport {
        module: "env".into(),
        name: "input".into(),
        params: vec![],
        results: vec![IrType::I64],
    });

    // ── 메인 함수 생성 ──
    let mut main_fn = IrFunction::new("main");
    main_fn.results.push(IrType::I64); // 반환: 최종 스택 top
    main_fn.is_export = true;

    // TVM 스택을 WASM 로컬로 시뮬레이션할 경우 로컬 할당
    // 간단한 경우: WASM 스택 직접 사용
    // 복잡한 경우: 로컬 변수 배열 사용

    // TVM 명령어 → IR 변환
    let import_count = module.imports.len() as u32;
    for inst in program {
        translate_instruction(&mut main_fn, inst, import_count);
    }

    // 만약 body가 비어있으면 기본 반환
    if main_fn.body.is_empty() {
        main_fn.body.push(IrOp::Const(0));
    }

    // 마지막이 Return이 아니면 추가
    if main_fn.body.last() != Some(&IrOp::Return) && main_fn.body.last() != Some(&IrOp::Halt) {
        // WASM에서는 함수 끝에서 자동으로 스택 top이 반환됨
        // Return을 명시적으로 넣지 않아도 됨
    }

    module.add_function(main_fn);
    module
}

/// 단일 TVM 명령어 → IR 변환
fn translate_instruction(func: &mut IrFunction, inst: &Instruction, import_count: u32) {
    let (sector, group, cmd) = (inst.addr.sector, inst.addr.group, inst.addr.command);

    // Sector 0 (코어) 명령어 매핑
    if sector == 0 {
        match (group, cmd) {
            // ── G0 논리 (비교) ──
            (0, 0) => func.body.push(IrOp::ConstTrit(1)),   // 참 → +1
            (0, 1) => func.body.push(IrOp::ConstTrit(-1)),  // 거짓 → -1
            (0, 2) => func.body.push(IrOp::ConstTrit(0)),   // 모름 → 0
            (0, 3) => func.body.push(IrOp::Eq),             // 같다
            (0, 4) => func.body.push(IrOp::Ne),             // 다르다
            (0, 5) => func.body.push(IrOp::Gt),             // 크다
            (0, 6) => func.body.push(IrOp::Lt),             // 작다
            (0, 7) => {
                // 아니다 (NOT) → 0 - val
                func.body.push(IrOp::TritNot);
            }
            (0, 8) => func.body.push(IrOp::TritAnd),        // 그리고 (AND)

            // ── G1 산술 ──
            (1, 0) => func.body.push(IrOp::Add),            // 더해
            (1, 1) => func.body.push(IrOp::Sub),            // 빼
            (1, 2) => func.body.push(IrOp::Mul),            // 곱해
            (1, 3) => func.body.push(IrOp::Div),            // 나눠
            (1, 4) => func.body.push(IrOp::Rem),            // 나머지
            (1, 5) => func.body.push(IrOp::Neg),            // 음수
            (1, 6) => func.body.push(IrOp::Abs),            // 절댓값
            (1, 7) => {
                // 제곱: x * x → dup + mul
                // WASM 스택에서: local.tee + local.get + mul
                let local_idx = func.params.len() as u32 + func.locals.len() as u32;
                func.locals.push(IrType::I64);
                func.body.push(IrOp::LocalSet(local_idx));
                func.body.push(IrOp::LocalGet(local_idx));
                func.body.push(IrOp::LocalGet(local_idx));
                func.body.push(IrOp::Mul);
            }
            (1, 8) => {
                // 제곱근: float 변환 → sqrt → 다시 int
                // WASM에 i64.sqrt 없음 → f64 경유
                func.body.push(IrOp::F64ConvertI64);
                // f64.sqrt = 0x9F
                func.body.push(IrOp::Nop); // placeholder, 직접 바이트 삽입 필요
                func.body.push(IrOp::I64TruncF64);
            }

            // ── G2 제어 ──
            (2, 0) => {
                // 점프 → br
                let target = inst.operands.first()
                    .and_then(|v| if let Value::Int(n) = v { Some(*n as u32) } else { None })
                    .unwrap_or(0);
                func.body.push(IrOp::Br(target));
            }
            (2, 1) => {
                // 조건점프 → br_if
                let target = inst.operands.first()
                    .and_then(|v| if let Value::Int(n) = v { Some(*n as u32) } else { None })
                    .unwrap_or(0);
                func.body.push(IrOp::BrIf(target));
            }
            (2, 2) => {
                // 호출 → call
                let idx = inst.operands.first()
                    .and_then(|v| if let Value::Int(n) = v { Some(*n as u32 + import_count) } else { None })
                    .unwrap_or(import_count);
                func.body.push(IrOp::Call(idx));
            }
            (2, 3) => func.body.push(IrOp::Return),         // 반환
            (2, 7) => func.body.push(IrOp::Halt),           // 종료
            (2, 8) => {
                // 비교 → 뺄셈 후 부호
                func.body.push(IrOp::Sub);
            }

            // ── G3 스택 ──
            (3, 0) => {
                // 넣어 (PUSH)
                let val = inst.operands.first().cloned().unwrap_or(Value::Int(0));
                match val {
                    Value::Int(n) => func.body.push(IrOp::Const(n)),
                    Value::Float(f) => func.body.push(IrOp::ConstF64(f)),
                    Value::Bool(b) => func.body.push(IrOp::Const(if b { 1 } else { -1 })),
                    Value::Trit(t) => func.body.push(IrOp::ConstTrit(t.to_i8())),
                    Value::Nil => func.body.push(IrOp::Const(0)),
                    Value::Str(s) => {
                        func.body.push(IrOp::Const(s.len() as i64));
                    }
                    _ => func.body.push(IrOp::Const(0)),
                }
            }
            (3, 1) => func.body.push(IrOp::Drop),           // 꺼내 (POP)
            (3, 2) => func.body.push(IrOp::Dup),            // 복사 (DUP)
            (3, 3) => func.body.push(IrOp::Swap),           // 바꿔 (SWAP)
            (3, 4) => {
                // 비움 (CLEAR) → 여러 drop
                func.body.push(IrOp::Drop);
            }
            (3, 5) => {
                // 보여줘 (PRINT) → call $print
                func.body.push(IrOp::CallImport(0)); // import[0] = print
            }
            (3, 6) => {
                // 입력해 (INPUT)
                func.body.push(IrOp::CallImport(2)); // import[2] = input
            }
            (3, 7) => {
                // 저장해 (STORE to global)
                let idx = inst.operands.first()
                    .and_then(|v| if let Value::Int(n) = v { Some(*n as u32) } else { None })
                    .unwrap_or(0);
                func.body.push(IrOp::GlobalSet(idx));
            }
            (3, 8) => {
                // 불러와 (LOAD from global)
                let idx = inst.operands.first()
                    .and_then(|v| if let Value::Int(n) = v { Some(*n as u32) } else { None })
                    .unwrap_or(0);
                func.body.push(IrOp::GlobalGet(idx));
            }

            // ── G8 힙/레지스터 ──
            (8, 3) => {
                // 할당 → memory.grow
                func.body.push(IrOp::MemGrow);
            }
            (8, 5) => {
                // 읽어 → memory load
                func.body.push(IrOp::MemLoad(0));
            }
            (8, 6) => {
                // 써 → memory store
                func.body.push(IrOp::MemStore(0));
            }

            _ => {
                // 미구현 → NOP
                func.body.push(IrOp::Nop);
            }
        }
    } else {
        // 다른 섹터는 아직 NOP
        func.body.push(IrOp::Nop);
    }
}

// ─────────────────────────────────────────────
// 전체 파이프라인: TVM → IR → WASM
// ─────────────────────────────────────────────

/// TVM 프로그램 → .wasm 바이너리 (전체 파이프라인)
pub fn compile_to_wasm(program: &[Instruction], module_name: &str) -> Vec<u8> {
    // Step 1: TVM → IR
    let ir = tvm_to_ir(program, module_name);

    // Step 2: IR → WASM binary
    WasmBuilder::build(&ir)
}

/// 한선어 소스 → .wasm 바이너리 (원스톱)
pub fn compile_source_to_wasm(source: &str, module_name: &str) -> Vec<u8> {
    let program = crate::assembler::assemble(source);
    compile_to_wasm(&program, module_name)
}

/// 컴파일 결과 정보
pub struct CompileResult {
    pub wasm_bytes: Vec<u8>,
    pub ir_op_count: usize,
    pub func_count: usize,
    pub import_count: usize,
}

/// 상세 컴파일 (정보 포함)
pub fn compile_with_info(source: &str, module_name: &str) -> CompileResult {
    let program = crate::assembler::assemble(source);
    let ir = tvm_to_ir(&program, module_name);
    let ir_ops: usize = ir.functions.iter().map(|f| f.body.len()).sum();
    let func_count = ir.functions.len();
    let import_count = ir.imports.len();
    let wasm = WasmBuilder::build(&ir);

    CompileResult {
        wasm_bytes: wasm,
        ir_op_count: ir_ops,
        func_count,
        import_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_compile() {
        let wasm = compile_source_to_wasm("넣어 5\n넣어 3\n더해\n종료", "test_add");
        assert_eq!(&wasm[0..4], b"\0asm");
        assert!(wasm.len() > 20);
        println!("add WASM: {} bytes", wasm.len());
    }

    #[test]
    fn test_compile_with_info() {
        let result = compile_with_info(
            "넣어 10\n넣어 20\n더해\n보여줘\n종료",
            "calc"
        );
        println!("WASM: {} bytes, IR ops: {}, funcs: {}, imports: {}",
            result.wasm_bytes.len(), result.ir_op_count,
            result.func_count, result.import_count);
        assert!(result.wasm_bytes.len() > 0);
        assert!(result.ir_op_count > 0);
    }

    #[test]
    fn test_trit_compile() {
        // 3진 논리 프로그램
        let wasm = compile_source_to_wasm("참\n모름\n그리고\n종료", "trit_logic");
        assert_eq!(&wasm[0..4], b"\0asm");
        println!("trit_logic WASM: {} bytes", wasm.len());
    }
}
