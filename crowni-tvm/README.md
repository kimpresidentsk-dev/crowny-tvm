# CROWNIN TVM v0.3.0

**균형3진 Meta-Kernel + 한선어 컴파일러 + 생태계 도구**

## 빠른 시작

```bash
cargo build --release
./target/release/crowni-tvm help
./target/release/crowni-tvm all     # 전체 데모
```

## 아키텍처

```
┌─────────────────────────────────────┐
│  한선어 (.cws) / SDK / 외부 앱      │
├─────────────────────────────────────┤
│  CAR (Crowny Application Runtime)   │
├─────────────────────────────────────┤
│  Meta-Kernel                        │
│    Scheduler + Permission           │
│    Transaction + Consensus          │
│    CTP Protocol + FPGA Bridge       │
├─────────────────────────────────────┤
│  TVM — 729 Opcodes (9×9×9)         │
├─────────────────────────────────────┤
│  Rust Host → macOS / Linux / WASM   │
└─────────────────────────────────────┘
```

## 26개 모듈 (11,300+ 줄)

| 계층 | 모듈 | 역할 |
|------|------|------|
| **코어** | trit, value, heap, opcode, vm | TVM 실행 엔진 |
| **명령어** | sectors (729개) | 9섹터 전체 opcode |
| **컴파일** | assembler, ir, wasm_gen, compiler | 소스→IR→WASM |
| **한선어** | hanseon | 렉서+파서+코드생성 |
| **바이트코드** | bytecode | .크라운 직렬화 |
| **커널** | kernel, scheduler, permission, transaction | Meta-Kernel |
| **네트워크** | network, bridge | CTP + FPGA |
| **런타임** | car | Application Runtime |
| **서비스** | webserver (Server + LLM) | 웹서버 + AI 호출 |
| **도구** | cpm, trit_test, debugger | 패키지/테스트/디버그 |
| **인프라** | trit_store, trit_log | 영속화 + 이벤트 로그 |

## CLI

```bash
crowni-tvm run <파일>       # .hsn 실행
crowni-tvm hanseon <파일>   # 한선어 컴파일+실행
crowni-tvm compile <파일>   # → .wasm
crowni-tvm bytecode <파일>  # → .크라운
crowni-tvm debug <파일>     # 디버거
crowni-tvm demo             # TVM 데모
crowni-tvm kernel           # Meta-Kernel
crowni-tvm car              # Application Runtime
crowni-tvm sectors          # 729 Opcode
crowni-tvm server           # 웹서버
crowni-tvm llm              # LLM 호출기
crowni-tvm all              # 전체 데모
```

## 한선어 예시

```crowny
; AI 3진 판단
질문해 "이 지원자를 채용할까?"

만약 {
    값 "합격"
    보여줘
} 보류 {
    값 "추가면접"
    보여줘
} 아니면 {
    값 "불합격"
    보여줘
}

끝
```

## 문서

- `docs/CROWNY-CANON-v1.0.md` — 전체 스펙 동결 문서
- `docs/EDUCATION-MANUAL.md` — 4단계 교육 매뉴얼

## IDE

- `ide/vscode/` — VSCode Extension (문법 하이라이팅 + Trit 시각화)

## CI/CD

- `.github/workflows/ci.yml` — 빌드/테스트/릴리즈 자동화

## 테스트

```bash
cargo test --release    # 86+ 테스트
```

---

**Architect: KPS (Han Seon) | Engine: Rust | Canon: v1.0 FROZEN**
