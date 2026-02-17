# crowny-sdk-go

Crowny 균형3진 플랫폼 Go SDK

## 설치

```bash
go get github.com/kimpresidentsk-dev/crowny-tvm/sdk/go
```

## 빠른 시작

```go
package main

import (
    "fmt"
    crowny "github.com/kimpresidentsk-dev/crowny-tvm/sdk/go"
)

func main() {
    client := crowny.NewClient("http://localhost:7293")

    // 한선어 실행
    result, _ := client.Run("넣어 42\n넣어 58\n더해\n종료")
    fmt.Println(result.State) // "P"

    // LLM 호출
    answer, _ := client.Ask("이 스타트업에 투자해야 할까?")
    fmt.Println(answer.State.ToKorean()) // "성공" | "보류" | "실패"

    // 3진 합의
    decision, _ := client.ConsensusCall("수술 진행?", []string{"claude", "gpt4", "gemini"})
    fmt.Println(decision.Consensus) // "P" | "O" | "T"
}
```

## Trit 연산

```go
crowny.TritNot(crowny.P)            // T
crowny.TritAnd(crowny.P, crowny.O)  // O
crowny.TritOr(crowny.T, crowny.P)   // P
crowny.Consensus([]crowny.TritValue{crowny.P, crowny.P, crowny.T}) // P
```
