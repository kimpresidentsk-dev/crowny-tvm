///! ┌───────────────────────────────────────────┐
///! │  CROWNIN TVM (크라우닌 균형3진법 가상머신)    │
///! │  한선어 명세 v1.0 | 9×9×9 = 729 Opcode     │
///! │  Architect: KPS (Han Seon)                │
///! └───────────────────────────────────────────┘
///!
///! 사용법:
///!   crowni-tvm                    → REPL 모드
///!   crowni-tvm run <file.hsn>     → 파일 실행
///!   crowni-tvm demo               → 내장 데모
///!   crowni-tvm info               → 명령어 목록
///!   crowni-tvm trit <decimal>     → 10진→균형3진 변환
///!   crowni-tvm decode <TOOPPT>    → 6트릿→opcode 디코딩

mod trit;
mod value;
mod heap;
mod opcode;
mod vm;
mod assembler;
mod scheduler;
mod permission;
mod transaction;
mod kernel;
mod network;
mod bridge;
mod ir;
mod wasm_gen;
mod compiler;
mod car;
mod bytecode;
mod sectors;
mod hanseon;
mod webserver;
mod cpm;
mod trit_test;
mod debugger;
mod trit_store;
mod trit_log;
mod node;
mod token;
mod wasm_node;
mod local_consensus;
mod industry;
mod platform;
mod browser;
mod website;
mod os;
mod chain;
mod live_consensus;
mod dex;
mod crossbridge;
mod nft;
mod contract_vm;

use std::env;
use std::fs;
use std::io::{self, Write};

use trit::Word6;
use vm::TVM;
use opcode::{SECTOR_NAMES, GROUP_NAMES_CORE};
use assembler::assemble;
use kernel::{CrownyKernel, KernelConfig};
use scheduler::{TritPriority, TritResult};
use permission::{TritPermission, Action};

const BANNER: &str = r#"
╔═══════════════════════════════════════════════════════╗
║   ██████╗██████╗  ██████╗ ██╗    ██╗███╗   ██╗       ║
║  ██╔════╝██╔══██╗██╔═══██╗██║    ██║████╗  ██║       ║
║  ██║     ██████╔╝██║   ██║██║ █╗ ██║██╔██╗ ██║       ║
║  ██║     ██╔══██╗██║   ██║██║███╗██║██║╚██╗██║       ║
║  ╚██████╗██║  ██║╚██████╔╝╚███╔███╔╝██║ ╚████║       ║
║   ╚═════╝╚═╝  ╚═╝ ╚═════╝  ╚══╝╚══╝ ╚═╝  ╚═══╝       ║
║                                                       ║
║  CROWNIN TVM v0.4.0 — 균형3진법 가상머신               ║
║  한선어 v1.0 | 9 섹터 × 729 Opcode | 티옴타 기계어     ║
║  Architect: KPS (Han Seon) | Engine: Rust              ║
╚═══════════════════════════════════════════════════════╝
"#;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        repl();
        return;
    }

    match args[1].as_str() {
        "run" => {
            if args.len() < 3 {
                eprintln!("사용법: crowni-tvm run <파일.hsn>");
                return;
            }
            run_file(&args[2]);
        }
        "demo" => run_demo(),
        "info" => show_info(),
        "trit" => {
            if args.len() < 3 {
                eprintln!("사용법: crowni-tvm trit <정수>");
                return;
            }
            convert_trit(&args[2]);
        }
        "decode" => {
            if args.len() < 3 {
                eprintln!("사용법: crowni-tvm decode <6트릿문자열>");
                return;
            }
            decode_trit_str(&args[2]);
        }
        "help" | "--help" | "-h" => show_help(),
        "kernel" | "커널" => run_kernel_demo(),
        "protocol" | "프로토콜" => run_protocol_demo(),
        "fpga" | "로드맵" => run_fpga_demo(),
        "wasm" | "와즘" => run_wasm_demo(),
        "car" | "런타임" => run_car_demo(),
        "sectors" | "섹터" => run_sectors_demo(),
        "hanseon" | "한선어" => {
            if args.len() >= 3 {
                compile_hanseon(&args[2]);
            } else {
                run_hanseon_demo();
            }
        }
        "server" | "서버" => run_server_demo(),
        "llm" | "호출기" => run_llm_demo(),
        "cpm" | "패키지" => run_cpm_demo(),
        "test" | "테스트" => run_test_demo(),
        "debug" | "디버그" => {
            if args.len() >= 3 {
                debug_file(&args[2]);
            } else {
                run_debug_demo();
            }
        }
        "store" | "영속화" => run_store_demo(),
        "log" | "로그" => run_log_demo(),
        "node" | "노드" => node::demo_distributed_node(),
        "token" | "토큰" => token::demo_token(),
        "wasm-node" | "브라우저노드" => wasm_node::demo_wasm_browser_node(),
        "consensus" | "합의" => local_consensus::demo_local_consensus(),
        "industry" | "산업" => industry::demo_industry(),
        "platform" | "플랫폼" => platform::demo_platform(),
        "browser" | "브라우저" => browser::demo_browser(),
        "website" | "웹사이트" => website::demo_website(),
        "os" | "운영체제" => os::demo_os(),
        "chain" | "체인" | "블록체인" => chain::demo_chain(),
        "live" | "라이브" | "live-consensus" => live_consensus::demo_live_consensus(),
        "dex" | "거래소" => dex::demo_dex(),
        "bridge" | "브릿지" => crossbridge::demo_bridge(),
        "nft" => nft::demo_nft(),
        "contract" | "스마트" | "sc" => contract_vm::demo_contract_vm(),
        "compile" | "컴파일" => {
            if args.len() < 3 {
                eprintln!("사용법: crowni-tvm compile <소스.hsn> [출력.wasm]");
                return;
            }
            let output = if args.len() >= 4 { &args[3] } else { "output.wasm" };
            compile_file(&args[2], output);
        }
        "bytecode" | "바이트코드" => {
            if args.len() < 3 {
                eprintln!("사용법: crowni-tvm bytecode <소스.hsn> [출력.크라운]");
                return;
            }
            let output = if args.len() >= 4 { &args[3] } else { "output.크라운" };
            bytecode_file(&args[2], output);
        }
        "all" | "전체" => {
            run_demo();
            println!("\n{}\n", "═".repeat(60));
            run_kernel_demo();
            println!("\n{}\n", "═".repeat(60));
            run_protocol_demo();
            println!("\n{}\n", "═".repeat(60));
            run_fpga_demo();
            println!("\n{}\n", "═".repeat(60));
            run_wasm_demo();
            println!("\n{}\n", "═".repeat(60));
            run_car_demo();
            println!("\n{}\n", "═".repeat(60));
            run_sectors_demo();
            println!("\n{}\n", "═".repeat(60));
            run_hanseon_demo();
            println!("\n{}\n", "═".repeat(60));
            run_server_demo();
            println!("\n{}\n", "═".repeat(60));
            run_llm_demo();
            println!("\n{}\n", "═".repeat(60));
            run_cpm_demo();
            println!("\n{}\n", "═".repeat(60));
            run_test_demo();
            println!("\n{}\n", "═".repeat(60));
            run_debug_demo();
            println!("\n{}\n", "═".repeat(60));
            run_store_demo();
            println!("\n{}\n", "═".repeat(60));
            run_log_demo();
            println!("\n{}\n", "═".repeat(60));
            node::demo_distributed_node();
            println!("\n{}\n", "═".repeat(60));
            token::demo_token();
            println!("\n{}\n", "═".repeat(60));
            wasm_node::demo_wasm_browser_node();
            println!("\n{}\n", "═".repeat(60));
            local_consensus::demo_local_consensus();
            println!("\n{}\n", "═".repeat(60));
            industry::demo_industry();
            println!("\n{}\n", "═".repeat(60));
            platform::demo_platform();
            println!("\n{}\n", "═".repeat(60));
            browser::demo_browser();
            println!("\n{}\n", "═".repeat(60));
            website::demo_website();
            println!("\n{}\n", "═".repeat(60));
            os::demo_os();
            println!("\n{}\n", "═".repeat(60));
            chain::demo_chain();
            println!("\n{}\n", "═".repeat(60));
            live_consensus::demo_live_consensus();
            println!("\n{}\n", "═".repeat(60));
            dex::demo_dex();
            println!("\n{}\n", "═".repeat(60));
            crossbridge::demo_bridge();
            println!("\n{}\n", "═".repeat(60));
            nft::demo_nft();
            println!("\n{}\n", "═".repeat(60));
            contract_vm::demo_contract_vm();
        }
        _ => {
            // 파일이면 실행
            if args[1].ends_with(".hsn") || args[1].ends_with(".한선") {
                run_file(&args[1]);
            } else {
                eprintln!("알 수 없는 명령: {}", args[1]);
                show_help();
            }
        }
    }
}

// ── REPL ──

fn repl() {
    println!("{}", BANNER);
    println!("REPL 모드 — 한글 또는 영문 명령어 입력 (종료: 'exit' 또는 Ctrl+C)");
    println!("명령: .stack .regs .heap .dump .debug .run .reset .info .help\n");

    let mut vm = TVM::new();
    let mut buffer = String::new();

    loop {
        print!("크라운> ");
        io::stdout().flush().unwrap_or(());

        let mut line = String::new();
        if io::stdin().read_line(&mut line).unwrap_or(0) == 0 {
            break; // EOF
        }
        let line = line.trim();

        if line.is_empty() { continue; }

        // 메타 명령어
        match line {
            "exit" | "quit" | "나가" | "종료해" => break,
            ".stack" | ".스택" => { vm.dump_stack(); continue; }
            ".regs" | ".레지스터" => { vm.dump_registers(); continue; }
            ".heap" | ".힙" => { vm.heap.dump(); continue; }
            ".dump" | ".덤프" => { vm.dump_all(); continue; }
            ".debug" | ".디버그" => {
                vm.debug = !vm.debug;
                println!("디버그 모드: {}", if vm.debug { "ON" } else { "OFF" });
                continue;
            }
            ".reset" | ".초기화" => {
                vm = TVM::new();
                println!("VM 초기화 완료");
                continue;
            }
            ".info" | ".정보" => { show_info(); continue; }
            ".help" | ".도움" => {
                println!("명령어: .stack .regs .heap .dump .debug .reset .info .help exit");
                continue;
            }
            _ => {}
        }

        // .run 으로 버퍼 실행
        if line == ".run" || line == ".실행" {
            if buffer.is_empty() {
                println!("버퍼가 비어있습니다. 명령어를 입력하세요.");
            } else {
                let program = assemble(&buffer);
                if !program.is_empty() {
                    println!("--- {} 명령어 실행 ---", program.len());
                    vm.load(program);
                    match vm.run() {
                        Ok(()) => println!("--- 정상 종료 ({}사이클) ---", vm.cycles),
                        Err(e) => println!("--- 오류: {} ---", e),
                    }
                }
                buffer.clear();
            }
            continue;
        }

        // 즉시 실행 모드: 한 줄을 바로 실행
        let program = assemble(line);
        if program.is_empty() {
            // 어셈블 실패 → 버퍼에 추가
            buffer.push_str(line);
            buffer.push('\n');
            continue;
        }

        // 즉시 실행
        vm.halted = false;
        let old_ip = vm.ip;
        let old_prog_len = vm.program.len();

        // 기존 프로그램 뒤에 추가 실행
        for inst in &program {
            vm.program.push(inst.clone());
        }
        vm.ip = old_prog_len;
        vm.halted = false;

        match vm.run() {
            Ok(()) => {}
            Err(vm::VmError::Halted) => {}
            Err(e) => println!("오류: {}", e),
        }
    }

    println!("\n안녕히. 크라우닌 TVM을 종료합니다.");
}

// ── 파일 실행 ──

fn run_file(path: &str) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("파일 읽기 실패 '{}': {}", path, e);
            return;
        }
    };

    let program = assemble(&source);
    if program.is_empty() {
        eprintln!("프로그램이 비어있습니다.");
        return;
    }

    println!("=== CROWNIN TVM — {} ({} 명령어) ===", path, program.len());
    let mut vm = TVM::new();
    vm.load(program);

    match vm.run() {
        Ok(()) => println!("\n=== 정상 종료 ({}사이클) ===", vm.cycles),
        Err(e) => eprintln!("\n=== 오류: {} ===", e),
    }
}

// ── 데모 ──

fn run_demo() {
    println!("{}", BANNER);
    println!("═══ 데모 1: 산술 (10 + 20 = 30) ═══");
    {
        let src = "넣어 10\n넣어 20\n더해\n보여줘\n종료";
        let mut vm = TVM::new();
        vm.load(assemble(src));
        let _ = vm.run();
    }

    println!("\n═══ 데모 2: 3진 논리 (참 AND 모름 = 모름(T)) ═══");
    {
        // 3진: P AND O → min(1,0) = O
        let src = "참\n모름\n그리고\n보여줘\n종료";
        let mut vm = TVM::new();
        vm.load(assemble(src));
        let _ = vm.run();
    }

    println!("\n═══ 데모 3: 문자열 연결 ═══");
    {
        let src = "넣어 \"한선\"\n넣어 \"어\"\n더해\n보여줘\n종료";
        let mut vm = TVM::new();
        vm.load(assemble(src));
        let _ = vm.run();
    }

    println!("\n═══ 데모 4: 힙 할당/읽기/해제 ═══");
    {
        let src = "넣어 999\n할당\n복사\n읽어\n보여줘\n해제\n종료";
        let mut vm = TVM::new();
        vm.load(assemble(src));
        let _ = vm.run();
    }

    println!("\n═══ 데모 5: 레지스터 R0에 저장/읽기 ═══");
    {
        let src = "넣어 42\n레지쓰기 0\n레지읽기 0\n보여줘\n종료";
        let mut vm = TVM::new();
        vm.load(assemble(src));
        let _ = vm.run();
    }

    println!("\n═══ 데모 6: 제곱근(√144 = 12) ═══");
    {
        let src = "넣어 144\n제곱근\n보여줘\n종료";
        let mut vm = TVM::new();
        vm.load(assemble(src));
        let _ = vm.run();
    }

    println!("\n═══ 데모 7: 균형3진 인코딩 테스트 ═══");
    {
        // opcode (0,1,0) = 코어/산술/더해 → Word6
        let w = Word6::encode_opcode(0, 1, 0);
        let (s, g, c) = w.decode_opcode();
        println!("  더해 = ({},{},{}) → 6트릿: {} → 10진: {}", s, g, c, w, w.to_decimal());

        let w2 = Word6::encode_opcode(4, 4, 4);
        println!("  중심(4,4,4) → {} → 10진: {} (Om의 중심)", w2, w2.to_decimal());
    }

    println!("\n═══ 모든 데모 완료 ═══");
}

// ── 명령어 목록 ──

fn show_info() {
    let opcodes = opcode::build_opcodes();
    println!("╔═══════════════════════════════════════════════╗");
    println!("║  CROWNIN TVM — 등록된 명령어 목록              ║");
    println!("║  729 슬롯 중 {} 개 구현                        ║", opcodes.len());
    println!("╚═══════════════════════════════════════════════╝");

    // 섹터별 출력
    for sec in 0..9u8 {
        let (kr, en) = SECTOR_NAMES[sec as usize];
        let count = opcodes.iter().filter(|(a, _)| a.sector == sec).count();
        if count == 0 { continue; }

        println!("\n── 섹터 {}: {} ({}) — {} 명령어 ──", sec, kr, en, count);

        for grp in 0..9u8 {
            let group_ops: Vec<_> = opcodes.iter()
                .filter(|(a, _)| a.sector == sec && a.group == grp)
                .collect();
            if group_ops.is_empty() { continue; }

            let grp_name = if sec == 0 && (grp as usize) < GROUP_NAMES_CORE.len() {
                GROUP_NAMES_CORE[grp as usize]
            } else {
                "─"
            };
            println!("  G{} [{}]:", grp, grp_name);
            for (addr, meta) in &group_ops {
                println!("    ({},{},{}) {:10} {:8} pop:{} push:{} oper:{}",
                    addr.sector, addr.group, addr.command,
                    meta.name_kr, meta.name_en,
                    meta.pops, meta.pushes, meta.operands);
            }
        }
    }
}

// ── 10진 → 균형3진 변환 ──

fn convert_trit(input: &str) {
    match input.parse::<i16>() {
        Ok(val) if (-364..=364).contains(&val) => {
            let w = Word6::from_decimal(val);
            println!("10진수:  {}", val);
            println!("균형3진: {} (6트릿)", w);
            let (s, g, c) = w.decode_opcode();
            println!("opcode:  ({},{},{}) = 섹터:{} 그룹:{} 명령:{}", s, g, c, s, g, c);
            println!("복원:    {}", w.to_decimal());
        }
        Ok(val) => eprintln!("6트릿 범위 초과: {} (허용: -364 ~ +364)", val),
        Err(e) => eprintln!("정수 파싱 실패: {} — {}", input, e),
    }
}

// ── 6트릿 문자열 → opcode 디코딩 ──

fn decode_trit_str(input: &str) {
    match Word6::from_trit_str(input) {
        Some(w) => {
            let (s, g, c) = w.decode_opcode();
            let opcodes = opcode::build_opcodes();
            let addr = opcode::OpcodeAddr::new(s, g, c);
            let name = opcodes.get(&addr).map(|m| format!("{} ({})", m.name_kr, m.name_en)).unwrap_or("(미등록)".into());
            println!("6트릿:   {}", w);
            println!("10진수:  {}", w.to_decimal());
            println!("opcode:  ({},{},{}) → {}", s, g, c, name);
        }
        None => eprintln!("6트릿 파싱 실패: '{}' (T/O/P 6문자 필요)", input),
    }
}

fn show_help() {
    println!("CROWNIN TVM v0.4.0 — 균형3진 Meta-Kernel + 생태계");
    println!();
    println!("사용법:");
    println!("  crowni-tvm                 REPL (대화형) 모드");
    println!("  crowni-tvm run <파일>       .hsn 파일 실행");
    println!("  crowni-tvm hanseon <파일>   한선어 컴파일+실행");
    println!("  crowni-tvm compile <파일>   .hsn → .wasm 컴파일");
    println!("  crowni-tvm bytecode <파일>  .hsn → .크라운 바이트코드");
    println!("  crowni-tvm debug <파일>     디버그 모드 실행");
    println!("  crowni-tvm demo            TVM 데모");
    println!("  crowni-tvm kernel          Meta-Kernel 데모");
    println!("  crowni-tvm protocol        CTP 프로토콜 데모");
    println!("  crowni-tvm fpga            FPGA 로드맵 데모");
    println!("  crowni-tvm wasm            WASM 변환 데모");
    println!("  crowni-tvm car             CAR (Application Runtime) 데모");
    println!("  crowni-tvm sectors         729 전체 섹터 데모");
    println!("  crowni-tvm hanseon         한선어 컴파일러 데모");
    println!("  crowni-tvm server          웹서버 데모");
    println!("  crowni-tvm llm             LLM 호출기 데모");
    println!("  crowni-tvm cpm             패키지 매니저 데모");
    println!("  crowni-tvm test            Trit 테스트 프레임워크 데모");
    println!("  crowni-tvm debug           디버거 데모");
    println!("  crowni-tvm store           영속화 레이어 데모");
    println!("  crowni-tvm log             이벤트 로그 데모");
    println!("  crowni-tvm node            분산 노드 데모");
    println!("  crowni-tvm token           3진 토큰 시스템 데모");
    println!("  crowni-tvm wasm-node       WASM 브라우저 노드 데모");
    println!("  crowni-tvm consensus       로컬 3진 합의 데모 (OpenClaw)");
    println!("  crowni-tvm industry        산업 적용 데모 (의료/교육/트레이딩)");
    println!("  crowni-tvm platform        통합 플랫폼 데모 (Git+Deploy+DB+Runtime+Web3)");
    println!("  crowni-tvm browser         3진 웹브라우저 데모");
    println!("  crowni-tvm website         3진 웹사이트 데모");
    println!("  crowni-tvm os              CrownyOS 데모 (프로세스/파일/쉘)");
    println!("  crowni-tvm chain           CrownyChain 블록체인 데모 (PoT)");
    println!("  crowni-tvm live            OpenClaw 실제 HTTP 합의 데모");
    println!("  crowni-tvm dex             CrownyDEX 탈중앙 거래소 데모");
    println!("  crowni-tvm bridge          CrownyBridge 크로스체인 브릿지 데모");
    println!("  crowni-tvm nft             CrownyNFT 마켓플레이스 데모");
    println!("  crowni-tvm contract        스마트 컨트랙트 VM 데모");
    println!("  crowni-tvm contract        스마트 컨트랙트 VM 데모");
    println!("  crowni-tvm all             전체 데모");
    println!("  crowni-tvm info            명령어 목록");
    println!("  crowni-tvm trit <정수>      10진→균형3진 변환");
    println!("  crowni-tvm decode <TTT>     6트릿→opcode 디코딩");
    println!("  crowni-tvm help            이 도움말");
}

// ═══════════════════════════════════════════════
// Crowny Meta-Kernel 데모
// ═══════════════════════════════════════════════

fn run_kernel_demo() {
    println!("{}", BANNER);
    println!("═══ Crowny Meta-Kernel 데모 ═══\n");

    // ── 커널 부팅 ──
    let mut kernel = CrownyKernel::boot(KernelConfig {
        debug: true,
        ..KernelConfig::default()
    });
    println!();

    // ═══ 1. 스케줄러 데모 ═══
    println!("━━━ 1. 3진 스케줄러 ━━━");
    println!("  태스크 상태: P(활성) O(대기) T(취소)");
    println!("  우선순위:    P(높음) O(보통) T(낮음)\n");

    let r1 = kernel.execute_task(
        "행렬계산", TritPriority::High,
        Box::new(|| { println!("  → [P높음] 행렬계산 실행!"); TritResult::Success }),
    );
    println!("  결과: {}\n", r1);

    let r2 = kernel.execute_task(
        "로그기록", TritPriority::Low,
        Box::new(|| { println!("  → [T낮음] 로그기록 실행!"); TritResult::Success }),
    );
    println!("  결과: {}\n", r2);

    let r3 = kernel.execute_task(
        "보류작업", TritPriority::Normal,
        Box::new(|| { println!("  → [O보통] 보류작업 → Pending 반환"); TritResult::Pending }),
    );
    println!("  결과: {} (재시도 큐에 등록됨)\n", r3);

    // ═══ 2. 권한 엔진 데모 ═══
    println!("━━━ 2. 3진 권한 엔진 ━━━");
    println!("  판정: P(허용) O(검토) T(차단)\n");

    // 추가 정책 등록
    kernel.permission.add_policy(
        "관리자", "*", Action::Admin,
        TritPermission::Allow, "관리자 전권",
    );
    kernel.permission.add_policy(
        "사용자", "코드", Action::Execute,
        TritPermission::Allow, "사용자 코드실행 허용",
    );

    let checks = [
        ("관리자", "시스템", Action::Admin),
        ("사용자", "코드", Action::Execute),
        ("사용자", "데이터", Action::Read),
        ("사용자", "데이터", Action::Write),
        ("사용자", "데이터", Action::Delete),
        ("손님", "비밀", Action::Read),
    ];

    for (sub, obj, act) in &checks {
        let perm = kernel.permission.check(sub, obj, *act);
        println!("  {}→{}.{} = {}", sub, obj, act, perm);
    }
    println!();

    // ═══ 3. 트랜잭션 엔진 데모 ═══
    println!("━━━ 3. 3진 트랜잭션 엔진 ━━━");
    println!("  상태: P(확정) O(보류) T(취소)\n");

    // Commit 시나리오
    println!("  [시나리오A: Commit]");
    let tx1 = kernel.transaction.begin("데이터 저장");
    kernel.transaction.set(tx1, "이름", "한선").unwrap();
    kernel.transaction.set(tx1, "시스템", "크라우닌").unwrap();
    kernel.transaction.set(tx1, "버전", "1.0").unwrap();
    let state1 = kernel.transaction.commit(tx1).unwrap();
    println!("  TX결과: {} → 이름={}, 시스템={}", state1,
        kernel.transaction.get("이름").unwrap_or("?"),
        kernel.transaction.get("시스템").unwrap_or("?"));
    println!();

    // Rollback 시나리오
    println!("  [시나리오B: Rollback]");
    let tx2 = kernel.transaction.begin("잘못된 변경");
    kernel.transaction.set(tx2, "이름", "해킹됨").unwrap();
    println!("  변경 중: 이름={}", kernel.transaction.get("이름").unwrap_or("?"));
    let state2 = kernel.transaction.rollback(tx2).unwrap();
    println!("  TX결과: {} → 이름={} (복원됨)", state2,
        kernel.transaction.get("이름").unwrap_or("?"));
    println!();

    // ═══ 4. 통합 보호 실행 데모 ═══
    println!("━━━ 4. 통합 보호 실행 (권한→트랜잭션→스케줄러) ━━━\n");

    // 허용되는 실행
    let gr1 = kernel.execute_guarded(
        "사용자", "코드", Action::Execute,
        "TVM프로그램", TritPriority::High,
        Box::new(|| {
            println!("  → TVM 프로그램 실행됨!");
            TritResult::Success
        }),
    );
    println!("  결과: {}\n", gr1);

    // 차단되는 실행
    let gr2 = kernel.execute_guarded(
        "사용자", "시스템", Action::Delete,
        "시스템삭제시도", TritPriority::High,
        Box::new(|| {
            println!("  → 이건 실행되면 안됨!");
            TritResult::Success
        }),
    );
    println!("  결과: {}\n", gr2);

    // 검토 상태 실행 (우선순위 강등됨)
    let gr3 = kernel.execute_guarded(
        "사용자", "설정", Action::Write,
        "설정변경", TritPriority::High,
        Box::new(|| {
            println!("  → 검토 후 실행 (우선순위 강등: P→O)");
            TritResult::Success
        }),
    );
    println!("  결과: {}\n", gr3);

    // ═══ 5. 합의 (Consensus) 데모 ═══
    println!("━━━ 5. 3진 합의 투표 ━━━\n");
    {
        use crate::transaction::TritConsensus;
        let votes1 = vec![TritConsensus::Approved, TritConsensus::Approved, TritConsensus::Rejected];
        println!("  투표 [승인, 승인, 거부] → {}", transaction::TransactionEngine::consensus(&votes1));

        let votes2 = vec![TritConsensus::Approved, TritConsensus::Holding, TritConsensus::Rejected];
        println!("  투표 [승인, 보류, 거부] → {}", transaction::TransactionEngine::consensus(&votes2));

        let votes3 = vec![TritConsensus::Rejected, TritConsensus::Rejected, TritConsensus::Holding];
        println!("  투표 [거부, 거부, 보류] → {}", transaction::TransactionEngine::consensus(&votes3));
    }
    println!();

    // ═══ 6. TVM 실행 ═══
    println!("━━━ 6. TVM 프로그램 실행 ━━━\n");
    match kernel.execute_program("넣어 3\n제곱\n넣어 4\n제곱\n더해\n제곱근\n보여줘\n종료") {
        Ok(()) => println!("  (3²+4²=25, √25=5 실행 완료)"),
        Err(e) => println!("  오류: {}", e),
    }
    println!();

    // ── 커널 상태 ──
    println!("━━━ 커널 전체 상태 ━━━");
    kernel.dump();

    // ── 종료 ──
    kernel.shutdown();
    println!("\n═══ Crowny Meta-Kernel 데모 완료 ═══");
}

// ═══════════════════════════════════════════════
// 균형3진 프로토콜 (CTP) 데모
// ═══════════════════════════════════════════════

fn run_protocol_demo() {
    use network::{TritBuffer, NetTrit, CtpMessage, MessageType, StatusCode};

    println!("{}", BANNER);
    println!("═══ Crowny Trit Protocol (CTP) 데모 ═══\n");

    // ── 1. 트릿 인코딩 ──
    println!("━━━ 1. Trit↔2bit 물리 매핑 ━━━");
    println!("  T(-1) = 0b00");
    println!("  O( 0) = 0b01");
    println!("  P(+1) = 0b10");
    println!("  무효   = 0b11 (패딩)\n");

    for t in [NetTrit::T, NetTrit::O, NetTrit::P] {
        let bits = t.to_2bit();
        let back = NetTrit::from_2bit(bits).unwrap();
        println!("  {} → 0b{:02b} → {} ✓", t, bits, back);
    }
    println!();

    // ── 2. TritBuffer 직렬화 ──
    println!("━━━ 2. TritBuffer 직렬화 ━━━");
    let mut buf = TritBuffer::new();
    buf.push_word6(42);
    buf.push_word6(100);
    buf.push_word6(-7);
    println!("  트릿: {} ({} trits)", buf, buf.len());
    let bytes = buf.to_bytes();
    println!("  바이트: {:?} ({} bytes)", bytes, bytes.len());
    println!("  압축률: {} trits → {} bytes ({}%)",
        buf.len(), bytes.len(),
        bytes.len() * 100 / buf.len().max(1));

    // 라운드트립
    let restored = TritBuffer::from_bytes(&bytes, buf.len());
    println!("  복원:  {} ✓", restored);
    println!("  값 확인: {} / {} / {}",
        restored.read_word6(0).unwrap(),
        restored.read_word6(6).unwrap(),
        restored.read_word6(12).unwrap());
    println!();

    // ── 3. CTP 메시지 ──
    println!("━━━ 3. CTP 메시지 생성 ━━━");

    let mut payload = TritBuffer::new();
    payload.push_word6(42);   // 데이터: 42
    payload.push_word6(365);  // 데이터: 365 (거의 최대)

    let msg = CtpMessage::request(payload);
    println!("  메시지: {}", msg);

    let serialized = msg.serialize();
    let total_trits = serialized.len();
    let total_bytes = serialized.to_bytes().len();
    println!("  직렬화: {} trits ({} bytes)", total_trits, total_bytes);
    println!("  패킷:  {}", serialized);
    println!();

    // ── 4. HTTP 헤더 ──
    println!("━━━ 4. HTTP 위 CTP 헤더 ━━━");
    let resp = CtpMessage::response(StatusCode::Success, TritBuffer::new());
    let headers = resp.to_http_headers();
    for (k, v) in &headers {
        println!("  {}: {}", k, v);
    }
    println!();

    // ── 5. 메시지 타입 시연 ──
    println!("━━━ 5. CTP 메시지 종류 (3진) ━━━");
    println!("  P(+1) = 요청 (Request)");
    println!("  O( 0) = 정보 (Info/Notification)");
    println!("  T(-1) = 응답 (Response)");
    println!();
    println!("  상태 코드:");
    println!("  P(+1) = 성공 (Success)");
    println!("  O( 0) = 중립 (Neutral/Processing)");
    println!("  T(-1) = 오류 (Error)");
    println!();

    // ── 6. TCP 서버/클라이언트 안내 ──
    println!("━━━ 6. CTP 네트워크 사용법 ━━━");
    println!("  서버: TritNetAdapter::start_server(\"127.0.0.1:7293\")");
    println!("  클라: TritNetAdapter::send_request(\"127.0.0.1:7293\", &msg)");
    println!("  포트: 7293 = 3^6 + 3^5 + ... (균형3진 의미)");
    println!();

    println!("═══ 프로토콜 데모 완료 ═══");
}

// ═══════════════════════════════════════════════
// FPGA 이전 로드맵 + 물리 매핑 데모
// ═══════════════════════════════════════════════

fn run_fpga_demo() {
    use bridge::*;

    println!("{}", BANNER);
    println!("═══ FPGA 이전 로드맵 + 물리 매핑 데모 ═══\n");

    // ── 로드맵 출력 ──
    print_roadmap();
    println!();

    // ── 1. Tryte (3 trit) ──
    println!("━━━ 1. Tryte (3 Trits = 6 bits) ━━━");
    for v in [-13, -5, 0, 7, 13] {
        let t = Tryte::from_decimal(v as i8);
        let packed = t.to_packed_byte();
        let restored = Tryte::from_packed_byte(packed);
        println!("  {} → {} → 0x{:02X} → {} ✓",
            v, t, packed, restored.to_decimal());
    }
    println!();

    // ── 2. TritWord (6 trit = opcode) ──
    println!("━━━ 2. TritWord (6 Trits = 12 bits) ━━━");
    for v in [-364, -42, 0, 42, 364] {
        let w = TritWord::from_decimal(v as i16);
        let packed = w.to_packed_u16();
        let restored = TritWord::from_packed_u16(packed);
        let (s, g, c) = w.decode_opcode();
        println!("  {:4} → {} → 0x{:04X} → {:4} opcode:({},{},{}) ✓",
            v, w, packed, restored.to_decimal(), s, g, c);
    }
    println!();

    // ── 3. TritDWord (12 trit) ──
    println!("━━━ 3. TritDWord (12 Trits = 24 bits) ━━━");
    for v in [-265720, -1234, 0, 9999, 265720] {
        let d = TritDWord::from_decimal(v);
        let bytes = d.to_packed_bytes();
        println!("  {:7} → {} → [{:02X},{:02X},{:02X}] ✓",
            v, d, bytes[0], bytes[1], bytes[2]);
    }
    println!();

    // ── 4. 레지스터 뱅크 ──
    println!("━━━ 4. FPGA 레지스터 뱅크 (9 × 12-trit) ━━━");
    let mut bank = FpgaRegisterBank::new();
    bank.regs[0] = TritDWord::from_decimal(42);       // R0 = 42
    bank.regs[1] = TritDWord::from_decimal(100);      // R1 = 100
    bank.regs[8] = TritDWord::from_decimal(-999);     // R8 = -999
    bank.pc = TritDWord::from_decimal(0);              // PC = 0
    bank.status = Tryte::from_decimal(1);              // 비교=P(양)
    bank.dump();
    println!("  패킹 크기: {} bytes", FpgaRegisterBank::packed_size());
    println!();

    // ── 5. 3진 메모리 ──
    println!("━━━ 5. 3진 메모리 (Trit-addressable) ━━━");
    let mut mem = TritMemory::new(729 * 6); // 729 words × 6 trits = 4374 trits
    mem.write_word(0, &TritWord::from_decimal(42));
    mem.write_word(6, &TritWord::from_decimal(100));
    mem.write_word(12, &TritWord::from_decimal(-364));
    mem.write_dword(100, &TritDWord::from_decimal(12345));

    println!("  쓰기: [0]=42, [6]=100, [12]=-364, [100]=12345");
    println!("  읽기: [0]={}, [6]={}, [12]={}, [100]={}",
        mem.read_word(0).to_decimal(),
        mem.read_word(6).to_decimal(),
        mem.read_word(12).to_decimal(),
        mem.read_dword(100).to_decimal());
    mem.dump(0, 120);
    println!();

    // ── 6. 크기 비교 ──
    println!("━━━ 6. 2진 vs 3진 저장 효율 ━━━");
    println!("  ┌──────────┬──────────┬──────────┬──────────┐");
    println!("  │   단위    │  Trits   │  2진 bits │  범위     │");
    println!("  ├──────────┼──────────┼──────────┼──────────┤");
    println!("  │  Trit    │    1     │    2     │  3가지    │");
    println!("  │  Tryte   │    3     │    6     │  27가지   │");
    println!("  │  Word    │    6     │   12     │  729가지  │");
    println!("  │  DWord   │   12     │   24     │ 531441   │");
    println!("  │  QWord   │   24     │   48     │ ~2.8억    │");
    println!("  └──────────┴──────────┴──────────┴──────────┘");
    println!();
    println!("  정보밀도: log₂(3) ≈ 1.585 bits/trit");
    println!("  2bit 매핑 효율: 1.585/2.0 = 79.2%");
    println!("  (FPGA 네이티브에서는 100%)\n");

    println!("═══ FPGA 이전 데모 완료 ═══");
}

// ═══════════════════════════════════════════════
// TVM → WASM 변환 데모
// ═══════════════════════════════════════════════

fn run_wasm_demo() {
    println!("{}", BANNER);
    println!("═══ TVM → WASM 변환 데모 ═══\n");
    println!("GPT Spec §3: TVM Bytecode → IR → WASM Module → .wasm binary\n");

    // ── 1. 간단한 산술 ──
    println!("━━━ 1. 산술 프로그램 → WASM ━━━");
    {
        let source = "넣어 5\n넣어 3\n더해\n종료";
        println!("  소스: 넣어 5 / 넣어 3 / 더해 / 종료");

        let result = compiler::compile_with_info(source, "산술");
        println!("  IR ops: {}", result.ir_op_count);
        println!("  WASM: {} bytes", result.wasm_bytes.len());
        println!("  함수: {} 개 (+ import {})", result.func_count, result.import_count);

        // Magic 확인
        assert_eq!(&result.wasm_bytes[0..4], b"\0asm");
        println!("  Magic: \\0asm ✓");
        println!("  Version: {} ✓", result.wasm_bytes[4]);
        print!("  Hex (처음 24): ");
        for b in result.wasm_bytes.iter().take(24) {
            print!("{:02X} ", b);
        }
        println!();
    }
    println!();

    // ── 2. 피타고라스 ──
    println!("━━━ 2. 피타고라스 (3²+4²=5²) → WASM ━━━");
    {
        let source = "넣어 3\n제곱\n넣어 4\n제곱\n더해\n보여줘\n종료";
        println!("  소스: 넣어 3 / 제곱 / 넣어 4 / 제곱 / 더해 / 보여줘 / 종료");

        let result = compiler::compile_with_info(source, "피타고라스");
        println!("  IR ops: {}", result.ir_op_count);
        println!("  WASM: {} bytes", result.wasm_bytes.len());

        // IR 변환 내용 보기
        let program = assembler::assemble(source);
        let ir_module = compiler::tvm_to_ir(&program, "피타고라스");
        println!("  IR 변환:");
        for (i, op) in ir_module.functions[0].body.iter().enumerate() {
            println!("    [{:2}] {:?}", i, op);
        }
    }
    println!();

    // ── 3. 3진 논리 ──
    println!("━━━ 3. 3진 논리 → WASM ━━━");
    {
        let source = "참\n모름\n그리고\n종료";
        println!("  소스: 참(+1) / 모름(0) / 그리고(AND) / 종료");

        let result = compiler::compile_with_info(source, "삼진논리");
        println!("  IR ops: {}", result.ir_op_count);
        println!("  WASM: {} bytes", result.wasm_bytes.len());

        let program = assembler::assemble(source);
        let ir_module = compiler::tvm_to_ir(&program, "삼진논리");
        println!("  IR 변환:");
        for (i, op) in ir_module.functions[0].body.iter().enumerate() {
            println!("    [{:2}] {:?}", i, op);
        }
    }
    println!();

    // ── 4. 비교 연산 ──
    println!("━━━ 4. 비교 연산 → WASM ━━━");
    {
        let source = "넣어 10\n넣어 20\n크다\n종료";
        println!("  소스: 넣어 10 / 넣어 20 / 크다 / 종료");

        let result = compiler::compile_with_info(source, "비교");
        println!("  IR ops: {}", result.ir_op_count);
        println!("  WASM: {} bytes", result.wasm_bytes.len());

        let program = assembler::assemble(source);
        let ir_module = compiler::tvm_to_ir(&program, "비교");
        println!("  IR 변환:");
        for (i, op) in ir_module.functions[0].body.iter().enumerate() {
            println!("    [{:2}] {:?}", i, op);
        }
    }
    println!();

    // ── 5. WASM 구조 분석 ──
    println!("━━━ 5. WASM 모듈 구조 ━━━");
    {
        let result = compiler::compile_with_info("넣어 42\n종료", "분석");
        let wasm = &result.wasm_bytes;
        println!("  전체 크기: {} bytes", wasm.len());
        println!("  구조:");
        println!("    [0..4]   Magic:   {:02X} {:02X} {:02X} {:02X} (\\0asm)",
            wasm[0], wasm[1], wasm[2], wasm[3]);
        println!("    [4..8]   Version: {}.{}.{}.{}",
            wasm[4], wasm[5], wasm[6], wasm[7]);

        // 섹션 파싱
        let mut pos = 8;
        while pos < wasm.len() {
            let sec_id = wasm[pos];
            pos += 1;
            if pos >= wasm.len() { break; }

            // LEB128 크기 읽기
            let mut size: usize = 0;
            let mut shift = 0;
            loop {
                if pos >= wasm.len() { break; }
                let byte = wasm[pos] as usize;
                pos += 1;
                size |= (byte & 0x7F) << shift;
                shift += 7;
                if byte & 0x80 == 0 { break; }
            }

            let sec_name = match sec_id {
                1 => "Type",
                2 => "Import",
                3 => "Function",
                5 => "Memory",
                6 => "Global",
                7 => "Export",
                8 => "Start",
                10 => "Code",
                _ => "Unknown",
            };
            println!("    Section {}: {} ({} bytes)", sec_id, sec_name, size);
            pos += size;
        }
    }
    println!();

    // ── 6. GPT Spec 매핑 요약 ──
    println!("━━━ 6. GPT Spec 변환 매핑 (81개 중 구현) ━━━");
    println!("  ┌──────────┬──────────────┬────────────────┐");
    println!("  │  TVM     │  IR          │  WASM          │");
    println!("  ├──────────┼──────────────┼────────────────┤");
    println!("  │ 넣어 N   │ Const(N)     │ i64.const N    │");
    println!("  │ 더해     │ Add          │ i64.add        │");
    println!("  │ 빼       │ Sub          │ i64.sub        │");
    println!("  │ 곱해     │ Mul          │ i64.mul        │");
    println!("  │ 나눠     │ Div          │ i64.div_s      │");
    println!("  │ 나머지   │ Rem          │ i64.rem_s      │");
    println!("  │ 같다     │ Eq           │ i64.eq         │");
    println!("  │ 다르다   │ Ne           │ i64.ne         │");
    println!("  │ 크다     │ Gt           │ i64.gt_s       │");
    println!("  │ 작다     │ Lt           │ i64.lt_s       │");
    println!("  │ 참       │ ConstTrit(1) │ i64.const 1    │");
    println!("  │ 거짓     │ ConstTrit(-1)│ i64.const -1   │");
    println!("  │ 모름     │ ConstTrit(0) │ i64.const 0    │");
    println!("  │ 제곱     │ Dup+Mul      │ local.tee+mul  │");
    println!("  │ 보여줘   │ CallImport(0)│ call $print    │");
    println!("  │ 종료     │ Halt         │ unreachable    │");
    println!("  │ 반환     │ Return       │ return         │");
    println!("  │ 그리고   │ TritAnd      │ min(a,b)       │");
    println!("  │ 아니다   │ TritNot      │ 0-val          │");
    println!("  └──────────┴──────────────┴────────────────┘");
    println!();

    // ── 7. 파이프라인 요약 ──
    println!("━━━ 7. 변환 파이프라인 ━━━");
    println!("  ┌──────────────┐");
    println!("  │ 한선어 소스    │  넣어 5 / 더해 / 종료");
    println!("  └──────┬───────┘");
    println!("         ↓ assembler");
    println!("  ┌──────────────┐");
    println!("  │ TVM Bytecode │  6-trit opcodes");
    println!("  └──────┬───────┘");
    println!("         ↓ compiler::tvm_to_ir()");
    println!("  ┌──────────────┐");
    println!("  │   Crowny IR  │  Const(5), Add, Halt");
    println!("  └──────┬───────┘");
    println!("         ↓ WasmBuilder::build()");
    println!("  ┌──────────────┐");
    println!("  │  .wasm 바이너리│  \\0asm + sections");
    println!("  └──────┬───────┘");
    println!("         ↓");
    println!("  ┌──────────────┐");
    println!("  │ 브라우저/WASI │  실행!");
    println!("  └──────────────┘");
    println!();

    println!("═══ WASM 변환 데모 완료 ═══");
}

// ═══════════════════════════════════════════════
// .hsn → .wasm 파일 컴파일
// ═══════════════════════════════════════════════

fn compile_file(input: &str, output: &str) {
    let source = match fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("파일 읽기 오류: {} — {}", input, e);
            return;
        }
    };

    let result = compiler::compile_with_info(&source, input);

    match fs::write(output, &result.wasm_bytes) {
        Ok(()) => {
            println!("✓ 컴파일 완료");
            println!("  입력: {}", input);
            println!("  출력: {} ({} bytes)", output, result.wasm_bytes.len());
            println!("  IR ops: {}", result.ir_op_count);
            println!("  함수: {} | imports: {}", result.func_count, result.import_count);
        }
        Err(e) => {
            eprintln!("파일 쓰기 오류: {} — {}", output, e);
        }
    }
}

// ═══════════════════════════════════════════════
// .hsn → .크라운 바이트코드 직결화
// ═══════════════════════════════════════════════

fn bytecode_file(input: &str, output: &str) {
    let source = match fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => { eprintln!("파일 읽기 오류: {} — {}", input, e); return; }
    };
    let program = assembler::assemble(&source);
    let bytes = bytecode::serialize(&program);
    match fs::write(output, &bytes) {
        Ok(()) => {
            let info = bytecode::analyze(&bytes).unwrap();
            println!("✓ 바이트코드 저장 완료");
            println!("  입력: {}", input);
            println!("  출력: {} ({} bytes)", output, info.byte_size);
            println!("  명령어: {} | 평균 {:.1} bytes/inst", info.instruction_count, info.avg_bytes_per_inst);
        }
        Err(e) => eprintln!("쓰기 오류: {}", e),
    }
}

// ═══════════════════════════════════════════════
// CAR (Crowny Application Runtime) 데모
// ═══════════════════════════════════════════════

fn run_car_demo() {
    println!("{}", BANNER);
    println!("═══ CAR (Crowny Application Runtime) 데모 ═══\n");

    let mut runtime = car::CrownyRuntime::new();

    // 1. 소스 실행
    println!("━━━ 1. 소스 실행 (CAR.submit) ━━━");
    let result = runtime.run_source("데모", "넣어 100\n넣어 200\n더해\n종료");
    println!("  결과: {} — {}", result.state, result.data);
    println!("  Task#{} ({}ms)", result.task_id, result.elapsed_ms);

    // 2. WASM 컴파일
    println!("\n━━━ 2. WASM 컴파일 (CAR.submit) ━━━");
    let result = runtime.compile_wasm("데모", "넣어 42\n종료");
    println!("  결과: {} — {}", result.state, result.data);

    // 3. 커스텀 Task
    println!("\n━━━ 3. 커스텀 Task ━━━");
    let task = car::AppTask::new(car::TaskType::System, "admin", "상태확인");
    let result = runtime.submit(task, |_t| {
        (car::TritState::Success, car::ResultData::Text("시스템 정상".into()))
    });
    println!("  결과: {} — {}", result.state, result.data);

    // 4. 상태
    println!();
    runtime.dump();
    println!("\n═══ CAR 데모 완료 ═══");
}

// ═══════════════════════════════════════════════
// 729 전체 섹터 데모
// ═══════════════════════════════════════════════

fn run_sectors_demo() {
    println!("{}", BANNER);
    println!("═══ 729 Opcode 전체 섹터 데모 ═══\n");

    let map = sectors::build_all_sectors();
    println!("총 등록: {} opcodes\n", map.len());

    let stats = sectors::sector_stats(&map);
    println!("┌────┬────────────┬──────┬──────┐");
    println!("│ ID │ 섹터        │ 등록  │ 활성  │");
    println!("├────┼────────────┼──────┼──────┤");
    for (s, name, total, active) in &stats {
        println!("│ {}  │ {:10} │  {}  │  {:2}  │", s, name, total, active);
    }
    println!("└────┴────────────┴──────┴──────┘");

    // 핵심 명령어 샘플
    println!("\n━━━ 섹터별 핵심 명령 ━━━");
    let samples = [
        (0,0,0,"참"), (0,1,0,"더해"), (0,2,7,"종료"),
        (1,0,0,"질문해"), (1,2,1,"행렬곱"), (1,4,3,"감정분석"),
        (2,0,0,"칩초기화"), (2,0,7,"삼진ALU"),
        (3,0,0,"캐시읽기"), (3,1,0,"GC실행"),
        (4,0,0,"연결"), (4,2,0,"JSON파싱"),
        (5,0,0,"해시"), (5,0,3,"암호화"),
        (6,0,0,"로그인"), (6,0,2,"토큰생성"),
        (7,0,0,"스택덤프"), (7,0,6,"타임스탬프"),
        (8,0,0,"플러그인"), (8,0,5,"WASM로드"),
    ];
    for (s,g,c,expected) in samples {
        let addr = crate::opcode::OpcodeAddr::new(s,g,c);
        if let Some(meta) = map.get(&addr) {
            println!("  ({},{},{}) {} [{}] — pop:{} push:{}",
                s,g,c, meta.name_kr, meta.name_en, meta.pops, meta.pushes);
        }
    }

    println!("\n═══ 섹터 데모 완료 ═══");
}

// ═══════════════════════════════════════════════
// 한선어 컴파일러 데모
// ═══════════════════════════════════════════════

fn run_hanseon_demo() {
    println!("{}", BANNER);
    println!("═══ 한선어 컴파일러 v0.1 데모 ═══\n");

    // 1. 기본 산술
    println!("━━━ 1. 기본 산술 ━━━");
    let source = "값 5\n값 3\n더\n보여줘\n끝";
    println!("  소스: {}", source.replace('\n', " / "));
    let out = hanseon::compile(source);
    println!("  명령어: {}개 | 에러: {} | 경고: {}",
        out.instructions.len(), out.errors.len(), out.warnings.len());

    // 2. 변수
    println!("\n━━━ 2. 변수 사용 ━━━");
    let source = "변수 x = 10\n변수 y = 20\nx\ny\n더\n보여줘\n끝";
    println!("  소스: {}", source.replace('\n', " / "));
    let out = hanseon::compile(source);
    println!("  변수: {} | 명령어: {}", out.variables, out.instructions.len());

    // 3. 3진 논리
    println!("\n━━━ 3. 3진 분기 ━━━");
    let source = "참\n만약 {\n값 1\n보여줘\n} 아니면 {\n값 0\n보여줘\n}\n끝";
    println!("  소스: {}", source.replace('\n', " / "));
    let out = hanseon::compile(source);
    println!("  명령어: {} | 에러: {}", out.instructions.len(), out.errors.len());

    // 4. 함수
    println!("\n━━━ 4. 함수 정의 ━━━");
    let source = "함수 계산 {\n값 7\n값 3\n곱\n보여줘\n}\n끝";
    println!("  소스: {}", source.replace('\n', " / "));
    let out = hanseon::compile(source);
    println!("  함수: {} | 명령어: {}", out.functions, out.instructions.len());

    // 5. LLM 호출 구문
    println!("\n━━━ 5. LLM 호출 (섹터1) ━━━");
    let source = "질문해 \"균형3진의 장점?\"\n보여줘\n끝";
    println!("  소스: {}", source.replace('\n', " / "));
    let out = hanseon::compile(source);
    let llm_ops = out.instructions.iter().filter(|i| i.addr.sector == 1).count();
    println!("  명령어: {} | LLM ops: {}", out.instructions.len(), llm_ops);

    // 6. 한선어 → WASM
    println!("\n━━━ 6. 한선어 → WASM 직접 변환 ━━━");
    let wasm = hanseon::compile_to_wasm("값 42\n더\n끝");
    println!("  WASM: {} bytes | Magic: {:?}", wasm.len(), &wasm[0..4]);

    // 7. 영어도 가능
    println!("\n━━━ 7. 이중 언어 지원 ━━━");
    let out = hanseon::compile("val 10\nval 20\nadd\nprint\nend");
    println!("  영어 소스: val 10 / val 20 / add / print / end");
    println!("  명령어: {} | 에러: {}", out.instructions.len(), out.errors.len());

    println!("\n═══ 한선어 컴파일러 데모 완료 ═══");
}

fn compile_hanseon(input: &str) {
    let source = match fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => { eprintln!("파일 읽기 오류: {}", e); return; }
    };
    let out = hanseon::compile(&source);
    if !out.errors.is_empty() {
        for e in &out.errors { eprintln!("  오류: {}", e); }
        return;
    }
    for w in &out.warnings { println!("  경고: {}", w); }

    println!("✓ 컴파일 완료 — {}개 명령어, {}개 변수, {}개 함수",
        out.instructions.len(), out.variables, out.functions);

    // TVM 실행
    let mut vm = vm::TVM::new();
    vm.load(out.instructions);
    match vm.run() {
        Ok(()) => println!("✓ 실행 완료"),
        Err(e) => eprintln!("실행 오류: {:?}", e),
    }
}

// ═══════════════════════════════════════════════
// 웹서버 데모
// ═══════════════════════════════════════════════

fn run_server_demo() {
    println!("{}", BANNER);
    println!("═══ Crowny 웹서버 데모 ═══\n");

    let mut server = webserver::create_demo_server();
    let mut car = car::CrownyRuntime::new();

    // 1. GET /
    println!("━━━ 1. GET / ━━━");
    let req = webserver::HttpRequest::new(webserver::HttpMethod::Get, "/")
        .with_ctp(webserver::CtpHeader::success());
    let resp = server.handle(&req, &mut car);
    println!("  Status: {} | CTP: {}", resp.status, resp.ctp);
    println!("  Body: {}", resp.body);

    // 2. POST /run
    println!("\n━━━ 2. POST /run (한선어 실행) ━━━");
    let req = webserver::HttpRequest::new(webserver::HttpMethod::Post, "/run")
        .with_body("넣어 7\n넣어 6\n곱해\n종료")
        .with_ctp(webserver::CtpHeader::success());
    let resp = server.handle(&req, &mut car);
    println!("  Status: {} | 결과: {}", resp.status, resp.trit_result.state);
    println!("  Body: {}", resp.body);

    // 3. POST /compile
    println!("\n━━━ 3. POST /compile (WASM) ━━━");
    let req = webserver::HttpRequest::new(webserver::HttpMethod::Post, "/compile")
        .with_body("넣어 99\n종료")
        .with_ctp(webserver::CtpHeader::success());
    let resp = server.handle(&req, &mut car);
    println!("  Status: {} | Body: {}", resp.status, resp.body);

    // 4. CTP 권한 거부
    println!("\n━━━ 4. CTP 권한 거부 ━━━");
    let req = webserver::HttpRequest::new(webserver::HttpMethod::Get, "/")
        .with_ctp(webserver::CtpHeader::failed());
    let resp = server.handle(&req, &mut car);
    println!("  Status: {} (403 = 권한 거부)", resp.status);

    // 5. 404
    println!("\n━━━ 5. 404 테스트 ━━━");
    let req = webserver::HttpRequest::new(webserver::HttpMethod::Get, "/없음")
        .with_ctp(webserver::CtpHeader::success());
    let resp = server.handle(&req, &mut car);
    println!("  Status: {}", resp.status);

    println!("\n  {}", server.stats());
    car.dump();
    println!("\n═══ 웹서버 데모 완료 ═══");
}

// ═══════════════════════════════════════════════
// LLM 호출기 데모
// ═══════════════════════════════════════════════

fn run_llm_demo() {
    println!("{}", BANNER);
    println!("═══ Crowny LLM 호출기 데모 ═══\n");

    let mut car = car::CrownyRuntime::new();
    let mut llm = webserver::CrownyLlm::new();

    // 1. 단일 호출
    println!("━━━ 1. 단일 LLM 호출 ━━━");
    let result = llm.ask("균형3진법의 장점은?", &mut car);
    println!("  상태: {} | Task#{}", result.state, result.task_id);
    println!("  응답: {}", result.data);

    // 2. 모델 선택 호출
    println!("\n━━━ 2. GPT-4 모델 호출 ━━━");
    let req = webserver::LlmRequest::new(webserver::LlmModel::Gpt4, "WASM이란?")
        .with_temp(0.3);
    let result = llm.call(req, &mut car);
    println!("  상태: {} | 응답: {}", result.state, result.data);

    // 3. 다중 모델 합의
    println!("\n━━━ 3. 다중 모델 합의 호출 ━━━");
    let result = llm.consensus_call(
        "Rust vs Go 선택?",
        &[webserver::LlmModel::Claude, webserver::LlmModel::Gpt4, webserver::LlmModel::Gemini],
        &mut car,
    );
    println!("  합의 결과: {}", result.state);
    if let car::ResultData::Text(t) = &result.data {
        for line in t.lines() {
            println!("    {}", line);
        }
    }

    println!("\n  {}", llm.stats());
    car.dump();
    println!("\n═══ LLM 호출기 데모 완료 ═══");
}

// ═══════════════════════════════════════════════
// CPM (Crowny Package Manager) 데모
// ═══════════════════════════════════════════════

fn run_cpm_demo() {
    println!("{}", BANNER);
    println!("═══ CPM (Crowny Package Manager) 데모 ═══\n");

    let mut cpm = cpm::CrownyPM::new();
    let (total, installed, categories) = cpm.stats();
    println!("레지스트리: {}개 패키지 | {}개 카테고리\n", total, categories);

    // 1. 검색
    println!("━━━ 1. 패키지 검색 ━━━");
    let results = cpm.search("crowny");
    for pkg in &results {
        println!("  {} v{} [{}] — {}", pkg.name, pkg.version, pkg.trust, pkg.description);
    }

    // 2. 의존성 트리
    println!("\n━━━ 2. 의존성 트리: crowny.medical ━━━");
    let tree = cpm.dep_tree("crowny.medical", 0);
    print!("{}", tree);

    // 3. 설치
    println!("\n━━━ 3. 패키지 설치 ━━━");
    let result = cpm.install("crowny.edu");
    println!("  상태: {:?}", result.state);
    println!("  설치됨: {:?}", result.installed);

    // 4. import 해석
    println!("\n━━━ 4. import 해석 ━━━");
    if let Some(exports) = cpm.resolve_import("crowny.ai") {
        println!("  crowny.ai exports: {:?}", exports);
    }

    // 5. import 구문 파싱
    println!("\n━━━ 5. import 구문 파싱 ━━━");
    let imports = vec![
        "가져와 crowny.ai { LlmCall, Consensus }",
        "import crowny.web { Server }",
        "가져와 crowny.crypto",
    ];
    for line in imports {
        if let Some((pkg, items)) = cpm::parse_import(line) {
            println!("  {} → {} {:?}", line, pkg, items);
        }
    }

    // 6. 매니페스트
    println!("\n━━━ 6. crowny.toml 생성 ━━━");
    let mut manifest = cpm::Manifest::new("my-crowny-app");
    manifest.version = cpm::Version::new(1, 0, 0);
    manifest.author = "KPS".to_string();
    manifest.description = "3진 AI 서비스".to_string();
    manifest.add_dep("crowny.core", ">=0.3.0");
    manifest.add_dep("crowny.ai", ">=0.1.0");
    manifest.add_dep("crowny.web", ">=0.1.0");
    println!("{}", manifest.to_toml());

    // 7. 설치 현황
    let (_, installed, _) = cpm.stats();
    println!("━━━ 설치 현황: {}개 ━━━", installed);
    for pkg in cpm.list_installed() {
        println!("  ✓ {} v{} [{}]", pkg.name, pkg.version, pkg.category);
    }

    println!("\n═══ CPM 데모 완료 ═══");
}

// ═══════════════════════════════════════════════
// Trit Test Framework 데모
// ═══════════════════════════════════════════════

fn run_test_demo() {
    println!("{}", BANNER);
    println!("═══ Trit Test Framework 데모 ═══\n");

    // 1. 코어 테스트
    println!("━━━ 1. 코어 TVM 테스트 ━━━");
    let result = trit_test::core_suite().run();
    print!("{}", result.report());

    // 2. 상태 전이 테스트
    println!("\n━━━ 2. 상태 전이 규칙 테스트 ━━━");
    let result = trit_test::transition_suite().run();
    print!("{}", result.report());

    // 3. CAR 통합 테스트
    println!("\n━━━ 3. CAR 통합 테스트 ━━━");
    let result = trit_test::car_suite().run();
    print!("{}", result.report());

    // 4. 합의 엔진 테스트
    println!("\n━━━ 4. 합의 엔진 테스트 ━━━");
    let result = trit_test::consensus_suite().run();
    print!("{}", result.report());

    // 5. 커스텀 테스트
    println!("\n━━━ 5. 커스텀 테스트 (피타고라스) ━━━");
    let mut suite = trit_test::TestSuite::new("피타고라스 검증");
    suite.add(trit_test::source_test("3²+4²=25", "넣어 3\n제곱\n넣어 4\n제곱\n더해\n종료", 25));
    suite.add(trit_test::source_test("5²=25", "넣어 5\n제곱\n종료", 25));
    let result = suite.run();
    print!("{}", result.report());

    println!("\n═══ Trit Test Framework 데모 완료 ═══");
}

// ═══════════════════════════════════════════════
// Trit Debugger 데모
// ═══════════════════════════════════════════════

fn run_debug_demo() {
    println!("{}", BANNER);
    println!("═══ Trit Debugger 데모 ═══\n");

    // 1. 전체 실행 + 트레이스
    println!("━━━ 1. 프로그램 트레이스 ━━━");
    let source = "넣어 5\n넣어 3\n더해\n넣어 2\n곱해\n종료";
    println!("  소스: {}", source.replace('\n', " / "));
    let mut dbg = debugger::TritDebugger::from_source(source);
    dbg.run_all();
    print!("{}", dbg.dump_trace());
    println!("  최종값: {:?}", dbg.result_value());

    // 2. 단계 실행
    println!("\n━━━ 2. 단계 실행 ━━━");
    let mut dbg = debugger::TritDebugger::from_source("넣어 10\n넣어 20\n더해\n종료");
    dbg.load();
    for _ in 0..3 {
        if let Ok(event) = dbg.step() {
            match event {
                debugger::DebugEvent::Execute { pc, name, stack_after, .. } => {
                    let top = stack_after.last().map(|s| s.as_str()).unwrap_or("-");
                    println!("  step@{}: {} → top={}", pc, name, top);
                }
                _ => {}
            }
        }
    }
    println!("  결과: {:?}", dbg.result_value());

    // 3. 브레이크포인트
    println!("\n━━━ 3. 브레이크포인트 ━━━");
    let mut dbg = debugger::TritDebugger::from_source("넣어 1\n넣어 2\n넣어 3\n더해\n더해\n종료");
    dbg.load();
    dbg.set_breakpoint(3); // 더해(첫번째)에 BP
    let events = dbg.run_to_breakpoint();
    println!("  BP@3 설정 → {}개 이벤트 후 정지", events.len());
    print!("{}", dbg.dump_stack());

    // 4. 프로그램 리스팅
    println!("━━━ 4. 프로그램 리스팅 ━━━");
    print!("{}", dbg.dump_program());

    // 5. 프로파일
    println!("━━━ 5. 프로파일 ━━━");
    let mut dbg = debugger::TritDebugger::from_source(
        "넣어 1\n넣어 2\n더해\n넣어 3\n더해\n넣어 4\n더해\n보여줘\n종료"
    );
    dbg.run_all();
    print!("{}", dbg.profile());

    // 6. 디버거 명령어
    println!("━━━ 6. 디버거 명령어 ━━━");
    print!("{}", debugger::debug_help());

    println!("\n═══ Trit Debugger 데모 완료 ═══");
}

fn debug_file(input: &str) {
    let source = match fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => { eprintln!("파일 읽기 오류: {}", e); return; }
    };
    let mut dbg = debugger::TritDebugger::from_source(&source);
    dbg.run_all();
    print!("{}", dbg.dump_trace());
    print!("{}", dbg.dump_stack());
    print!("{}", dbg.profile());
    println!("최종값: {:?}", dbg.result_value());
}

// ═══════════════════════════════════════════════
// Trit Persistent Layer 데모
// ═══════════════════════════════════════════════

fn run_store_demo() {
    println!("{}", BANNER);
    println!("═══ Trit Persistent Layer 데모 ═══\n");

    let mut store = trit_store::TritStore::new();

    // 1. 기본 CRUD
    println!("━━━ 1. 기본 CRUD ━━━");
    store.set("서비스명", trit_store::StoreValue::Text("Crowny AI".into()));
    store.set("버전", trit_store::StoreValue::Int(4));
    store.set("활성화", trit_store::StoreValue::Trit(1));
    store.set("정확도", trit_store::StoreValue::Float(0.9731));
    println!("  저장: 4개 항목");
    println!("  크기: ~{} bytes", store.estimated_size());

    // 2. Trit 상태 인덱싱
    println!("\n━━━ 2. Trit 상태 인덱싱 ━━━");
    store.set("task_compile", trit_store::StoreValue::Text("컴파일 완료".into()));
    store.set("task_deploy", trit_store::StoreValue::Text("배포 대기".into()));
    store.set("task_test", trit_store::StoreValue::Text("테스트 실패".into()));
    store.set_trit_state("task_compile", 1);
    store.set_trit_state("task_deploy", 0);
    store.set_trit_state("task_test", -1);

    let (p, o, t) = store.trit_stats();
    println!("  상태: P:{} O:{} T:{}", p, o, t);
    println!("  P 목록: {:?}", store.filter_by_trit(1));
    println!("  T 목록: {:?}", store.filter_by_trit(-1));

    // 3. 트랜잭션
    println!("\n━━━ 3. 트랜잭션 ━━━");
    store.begin();
    store.set("tx_data1", trit_store::StoreValue::Int(100));
    store.set("tx_data2", trit_store::StoreValue::Int(200));
    store.commit();
    println!("  커밋 후: {}개 항목", store.len());

    store.begin();
    store.set("tx_rollback", trit_store::StoreValue::Int(999));
    store.rollback();
    println!("  롤백 후: {}개 (tx_rollback 없음: {})", store.len(), !store.exists("tx_rollback"));

    // 4. Snapshot
    println!("\n━━━ 4. Snapshot/복구 ━━━");
    let snap_id = store.snapshot();
    println!("  Snapshot#{} 생성 ({}개 항목)", snap_id, store.len());

    store.set("임시", trit_store::StoreValue::Text("삭제될 데이터".into()));
    store.delete("버전");
    println!("  변경 후: {}개 항목", store.len());

    store.restore(snap_id);
    println!("  복구 후: {}개 항목 (원래 상태)", store.len());

    // 5. 네임스페이스
    println!("\n━━━ 5. 네임스페이스 ━━━");
    let mut ns = trit_store::NamespacedStore::new();
    ns.get_or_create("crowny.ai").set("model", trit_store::StoreValue::Text("Claude".into()));
    ns.get_or_create("crowny.web").set("port", trit_store::StoreValue::Int(7293));
    ns.get_or_create("crowny.token").set("supply", trit_store::StoreValue::Int(729_000_000));
    println!("  네임스페이스: {}개 | 총 항목: {}", ns.namespaces().len(), ns.total_entries());

    // 6. 통계
    println!("\n━━━ 6. 통계 ━━━");
    println!("  {}", store.stats());
    println!("  WAL: {}개 엔트리", store.wal_len());

    println!("\n═══ Trit Persistent Layer 데모 완료 ═══");
}

// ═══════════════════════════════════════════════
// Trit Event Log 데모
// ═══════════════════════════════════════════════

fn run_log_demo() {
    println!("{}", BANNER);
    println!("═══ Trit Event Log (Observability) 데모 ═══\n");

    let mut log = trit_log::TritEventLog::new();

    // 알림 규칙 등록
    log.add_alert(trit_log::AlertRule::new("에러감지", trit_log::Category::Task, trit_log::Level::Error));
    log.add_alert(trit_log::AlertRule::new("권한거부", trit_log::Category::Permission, trit_log::Level::Warn)
        .with_trit(car::TritState::Failed));

    // 1. 시스템 이벤트
    println!("━━━ 1. 시스템 이벤트 ━━━");
    log.info(trit_log::Category::System, "kernel", "Meta-Kernel 부팅 완료", car::TritState::Success);
    log.info(trit_log::Category::System, "CAR", "Application Runtime 시작", car::TritState::Success);

    // 2. Task 라이프사이클
    println!("━━━ 2. Task 추적 ━━━");
    log.task_start(1, "한선어 컴파일");
    log.increment("task.count");
    log.record("task.latency", 12.5);
    log.task_end(1, car::TritState::Success);

    log.task_start(2, "WASM 변환");
    log.increment("task.count");
    log.record("task.latency", 8.1);
    log.task_end(2, car::TritState::Success);

    log.task_start(3, "LLM 호출");
    log.increment("task.count");
    log.record("task.latency", 350.0);
    log.error(trit_log::Category::Task, "LLM", "API 타임아웃");

    // 3. 합의 기록
    println!("━━━ 3. 합의 기록 ━━━");
    log.consensus_vote(1, "Claude", 1);
    log.consensus_vote(1, "GPT-4", 1);
    log.consensus_vote(1, "Gemini", -1);
    log.info(trit_log::Category::Consensus, "engine", "합의 결과: P (2:1)", car::TritState::Success);

    // 4. 권한 감사
    println!("━━━ 4. 권한 감사 ━━━");
    log.permission_check("user:admin", "kernel:execute", true);
    log.permission_check("user:guest", "kernel:shutdown", false);
    log.permission_check("app:web", "llm:call", true);

    // 5. 상태 전이
    println!("━━━ 5. 상태 전이 ━━━");
    log.set_min_level(trit_log::Level::Debug);
    log.state_transition("task_deploy", 0, 1);  // O → P
    log.state_transition("task_test", -1, 0);   // T → O (복구)
    log.state_transition("service", 1, 0);      // P → O (경고)

    // 6. 메트릭
    println!("━━━ 6. 메트릭 ━━━");
    log.gauge("cpu_usage", 37.5);
    log.gauge("memory_mb", 128.4);
    log.gauge("active_tasks", 3.0);

    // 7. 최근 이벤트
    println!("\n━━━ 7. 최근 이벤트 ━━━");
    print!("{}", log.dump_recent(10));

    // 8. 요약 보고서
    println!("━━━ 8. 요약 보고서 ━━━");
    print!("{}", log.summary());

    println!("\n═══ Trit Event Log 데모 완료 ═══");
}
