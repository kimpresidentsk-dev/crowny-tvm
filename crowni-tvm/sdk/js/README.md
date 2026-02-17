# @crowny/sdk

Crowny 균형3진 플랫폼 JavaScript/TypeScript SDK

## 설치

```bash
npm install @crowny/sdk
```

## 빠른 시작

```typescript
import { CrownyClient, Trit } from '@crowny/sdk';

// 서버 연결
const crowny = new CrownyClient('http://localhost:7293');

// 한선어 실행
const result = await crowny.run('넣어 42\n넣어 58\n더해\n종료');
console.log(result.state);  // 'P'
console.log(result.data);   // 100

// LLM 호출
const answer = await crowny.ask('이 스타트업에 투자해야 할까?');
console.log(Trit.toKorean(answer.state));  // '성공' | '보류' | '실패'

// 3진 합의 (3개 AI)
const decision = await crowny.consensus('수술을 진행해야 하는가?');
console.log(decision.consensus);  // 'P' | 'O' | 'T'
console.log(decision.trits);      // ['P', 'O', 'P']
```

## 로컬 실행 (서버 불필요)

```typescript
import { CrownyLocal } from '@crowny/sdk';

const local = new CrownyLocal();
const result = local.execute('넣어 7\n넣어 6\n곱해\n종료');
console.log(result.data);  // 42
```

## Trit 연산

```typescript
import { Trit } from '@crowny/sdk';

Trit.not('P');           // 'T'
Trit.and('P', 'O');      // 'O'
Trit.or('T', 'P');       // 'P'
Trit.consensus(['P','P','T']);  // 'P'
Trit.toKorean('O');      // '보류'
```
