# crowny-sdk

Crowny 균형3진 플랫폼 Rust SDK

## 설치

```toml
[dependencies]
crowny-sdk = "1.0"
```

## 빠른 시작

```rust
use crowny_sdk::{CrownyClient, Trit};

let mut client = CrownyClient::new("http://localhost:7293");

// 한선어 실행
let result = client.run("넣어 42\n종료");
assert_eq!(result.state, Trit::P);

// LLM 호출
let answer = client.ask("이 스타트업에 투자해야 할까?");
println!("{}", answer.state.to_korean());

// 3진 합의
let decision = client.consensus_call("수술 진행?", &["claude", "gpt4", "gemini"]);
println!("합의: {}", decision.consensus);
```

## Trit 연산

```rust
use crowny_sdk::Trit;

Trit::P.not();              // T
Trit::P.and(Trit::O);       // O
Trit::T.or(Trit::P);        // P
Trit::consensus(&[Trit::P, Trit::P, Trit::T]);  // P
Trit::P.to_korean();        // "성공"
```
