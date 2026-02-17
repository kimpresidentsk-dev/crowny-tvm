/**
 * @crowny/sdk — Crowny 균형3진 플랫폼 JavaScript SDK
 *
 * 사용법:
 *   import { CrownyClient, Trit } from '@crowny/sdk';
 *
 *   const crowny = new CrownyClient('http://localhost:7293');
 *   const result = await crowny.submit('execute', '넣어 42\n종료');
 *   console.log(result.state);  // 'P' | 'O' | 'T'
 */

// ═══════════════════════════════════════════════
// Trit 타입
// ═══════════════════════════════════════════════

export type TritValue = 'P' | 'O' | 'T';

export const Trit = {
  P: 'P' as TritValue,  // +1 성공
  O: 'O' as TritValue,  //  0 보류
  T: 'T' as TritValue,  // -1 실패

  /** 숫자 → Trit */
  from(n: number): TritValue {
    if (n > 0) return 'P';
    if (n < 0) return 'T';
    return 'O';
  },

  /** Trit → 숫자 */
  toNumber(t: TritValue): number {
    return t === 'P' ? 1 : t === 'T' ? -1 : 0;
  },

  /** Trit → 한국어 */
  toKorean(t: TritValue): string {
    return t === 'P' ? '성공' : t === 'T' ? '실패' : '보류';
  },

  /** NOT */
  not(t: TritValue): TritValue {
    return t === 'P' ? 'T' : t === 'T' ? 'P' : 'O';
  },

  /** AND (min) */
  and(a: TritValue, b: TritValue): TritValue {
    const na = Trit.toNumber(a), nb = Trit.toNumber(b);
    return Trit.from(Math.min(na, nb));
  },

  /** OR (max) */
  or(a: TritValue, b: TritValue): TritValue {
    const na = Trit.toNumber(a), nb = Trit.toNumber(b);
    return Trit.from(Math.max(na, nb));
  },

  /** 다수결 합의 */
  consensus(trits: TritValue[]): TritValue {
    const p = trits.filter(t => t === 'P').length;
    const t = trits.filter(t => t === 'T').length;
    if (p > t) return 'P';
    if (t > p) return 'T';
    return 'O';
  },
} as const;

// ═══════════════════════════════════════════════
// TritResult
// ═══════════════════════════════════════════════

export interface TritResult {
  state: TritValue;
  data: any;
  elapsed_ms: number;
  task_id: number;
}

// ═══════════════════════════════════════════════
// CTP Header
// ═══════════════════════════════════════════════

export class CtpHeader {
  trits: TritValue[];

  constructor(trits: TritValue[] = ['O','O','O','O','O','O','O','O','O']) {
    this.trits = trits.slice(0, 9);
    while (this.trits.length < 9) this.trits.push('O');
  }

  static success(): CtpHeader {
    return new CtpHeader(['P','P','P','O','O','O','O','O','O']);
  }

  static failed(): CtpHeader {
    return new CtpHeader(['T','T','T','O','O','O','O','O','O']);
  }

  static fromString(s: string): CtpHeader {
    const trits = s.split('').map(c => {
      if (c === 'P' || c === '+' || c === '1') return 'P' as TritValue;
      if (c === 'T' || c === '-') return 'T' as TritValue;
      return 'O' as TritValue;
    });
    return new CtpHeader(trits);
  }

  toString(): string {
    return this.trits.join('');
  }

  get state(): TritValue { return this.trits[0]; }
  get permission(): TritValue { return this.trits[1]; }
  get consensus(): TritValue { return this.trits[2]; }

  get overallState(): TritValue {
    if (this.trits.includes('T')) return 'T';
    if (this.trits.every(t => t === 'P')) return 'P';
    return 'O';
  }
}

// ═══════════════════════════════════════════════
// Task 타입
// ═══════════════════════════════════════════════

export type TaskType = 'compile' | 'execute' | 'web' | 'llm' | 'db' | 'file' | 'system';

export interface AppTask {
  type: TaskType;
  subject: string;
  payload: string;
  params?: Record<string, string>;
}

// ═══════════════════════════════════════════════
// CrownyClient
// ═══════════════════════════════════════════════

export interface CrownyConfig {
  baseUrl?: string;
  timeout?: number;
  ctp?: CtpHeader;
  onTritResult?: (result: TritResult) => void;
}

export class CrownyClient {
  private baseUrl: string;
  private timeout: number;
  private ctp: CtpHeader;
  private history: TritResult[] = [];
  private taskCounter = 0;
  private onTritResult?: (result: TritResult) => void;

  constructor(config: CrownyConfig | string = {}) {
    if (typeof config === 'string') {
      this.baseUrl = config;
      this.timeout = 30000;
      this.ctp = CtpHeader.success();
    } else {
      this.baseUrl = config.baseUrl || 'http://localhost:7293';
      this.timeout = config.timeout || 30000;
      this.ctp = config.ctp || CtpHeader.success();
      this.onTritResult = config.onTritResult;
    }
  }

  // ── 핵심: submit ──

  async submit(task: AppTask): Promise<TritResult> {
    const start = Date.now();
    this.taskCounter++;
    const taskId = this.taskCounter;

    try {
      const response = await this.request('POST', '/run', {
        type: task.type,
        subject: task.subject,
        payload: task.payload,
        params: task.params || {},
      });

      const result: TritResult = {
        state: this.parseTrit(response.상태 || response.state || 'O'),
        data: response.결과 || response.data || response,
        elapsed_ms: Date.now() - start,
        task_id: taskId,
      };

      this.history.push(result);
      this.onTritResult?.(result);
      return result;
    } catch (err) {
      const result: TritResult = {
        state: 'T',
        data: { error: err instanceof Error ? err.message : String(err) },
        elapsed_ms: Date.now() - start,
        task_id: taskId,
      };
      this.history.push(result);
      this.onTritResult?.(result);
      return result;
    }
  }

  // ── 편의 메서드 ──

  /** 한선어 소스 실행 */
  async run(source: string): Promise<TritResult> {
    return this.submit({ type: 'execute', subject: 'sdk', payload: source });
  }

  /** WASM 컴파일 */
  async compile(source: string): Promise<TritResult> {
    return this.submit({ type: 'compile', subject: 'sdk', payload: source });
  }

  /** LLM 호출 */
  async ask(prompt: string, model?: string): Promise<TritResult> {
    return this.submit({
      type: 'llm',
      subject: model || 'claude',
      payload: prompt,
    });
  }

  /** 다중 LLM 합의 호출 */
  async consensus(prompt: string, models: string[] = ['claude', 'gpt4', 'gemini']): Promise<ConsensusResult> {
    const start = Date.now();
    const results = await Promise.all(
      models.map(model => this.ask(prompt, model))
    );

    const trits = results.map(r => r.state);
    const final = Trit.consensus(trits);

    return {
      consensus: final,
      models: results.map((r, i) => ({ model: models[i], result: r })),
      trits,
      ctp: new CtpHeader([final, ...trits, 'O', 'O', 'O', 'O', 'O']),
      elapsed_ms: Date.now() - start,
    };
  }

  // ── 서버 상태 ──

  async ping(): Promise<TritResult> {
    const start = Date.now();
    try {
      const res = await this.request('GET', '/');
      return {
        state: 'P',
        data: res,
        elapsed_ms: Date.now() - start,
        task_id: 0,
      };
    } catch {
      return { state: 'T', data: 'unreachable', elapsed_ms: Date.now() - start, task_id: 0 };
    }
  }

  /** 히스토리 */
  getHistory(): TritResult[] {
    return [...this.history];
  }

  /** 통계 */
  getStats(): { total: number; p: number; o: number; t: number } {
    return {
      total: this.history.length,
      p: this.history.filter(r => r.state === 'P').length,
      o: this.history.filter(r => r.state === 'O').length,
      t: this.history.filter(r => r.state === 'T').length,
    };
  }

  // ── HTTP ──

  private async request(method: string, path: string, body?: any): Promise<any> {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeout);

    try {
      const res = await fetch(`${this.baseUrl}${path}`, {
        method,
        headers: {
          'Content-Type': 'application/json',
          'X-Crowny-Trit': this.ctp.toString(),
          'X-Crowny-Version': '1.0',
        },
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal,
      });

      // CTP 응답 헤더 파싱
      const ctpHeader = res.headers.get('X-Crowny-Trit');
      if (ctpHeader) {
        this.ctp = CtpHeader.fromString(ctpHeader);
      }

      return await res.json();
    } finally {
      clearTimeout(timer);
    }
  }

  private parseTrit(s: string): TritValue {
    if (typeof s === 'string') {
      if (s.includes('P') || s.includes('성공') || s.includes('Success')) return 'P';
      if (s.includes('T') || s.includes('실패') || s.includes('Failed')) return 'T';
    }
    return 'O';
  }
}

// ═══════════════════════════════════════════════
// 합의 결과
// ═══════════════════════════════════════════════

export interface ConsensusResult {
  consensus: TritValue;
  models: { model: string; result: TritResult }[];
  trits: TritValue[];
  ctp: CtpHeader;
  elapsed_ms: number;
}

// ═══════════════════════════════════════════════
// 로컬 실행 (서버 없이 SDK 단독)
// ═══════════════════════════════════════════════

export class CrownyLocal {
  private stack: any[] = [];
  private vars: Map<string, any> = new Map();

  /** 간단한 한선어 로컬 실행 */
  execute(source: string): TritResult {
    const start = Date.now();
    this.stack = [];

    try {
      const lines = source.split('\n').map(l => l.trim()).filter(l => l && !l.startsWith(';'));

      for (const line of lines) {
        if (line === '종료' || line === 'end' || line === '끝') break;

        const parts = line.split(/\s+/);
        const cmd = parts[0];

        switch (cmd) {
          case '넣어': case 'push': case '값': case 'val': {
            const val = parts.slice(1).join(' ');
            const num = Number(val);
            this.stack.push(isNaN(num) ? val.replace(/"/g, '') : num);
            break;
          }
          case '더해': case 'add': case '더': {
            const b = this.stack.pop() || 0, a = this.stack.pop() || 0;
            this.stack.push(a + b);
            break;
          }
          case '빼': case 'sub': {
            const b = this.stack.pop() || 0, a = this.stack.pop() || 0;
            this.stack.push(a - b);
            break;
          }
          case '곱해': case 'mul': case '곱': {
            const b = this.stack.pop() || 0, a = this.stack.pop() || 0;
            this.stack.push(a * b);
            break;
          }
          case '나눠': case 'div': {
            const b = this.stack.pop() || 1, a = this.stack.pop() || 0;
            this.stack.push(b !== 0 ? a / b : 0);
            break;
          }
          case '보여줘': case 'print':
            console.log('[Crowny]', this.stack[this.stack.length - 1]);
            break;
          case '복사': case 'dup':
            if (this.stack.length > 0) this.stack.push(this.stack[this.stack.length - 1]);
            break;
          case '바꿔': case 'swap': {
            const len = this.stack.length;
            if (len >= 2) [this.stack[len-1], this.stack[len-2]] = [this.stack[len-2], this.stack[len-1]];
            break;
          }
          case '참': case 'P': this.stack.push(1); break;
          case '모름': case 'O': this.stack.push(0); break;
          case '거짓': case 'T': this.stack.push(-1); break;
        }
      }

      const top = this.stack[this.stack.length - 1];
      return {
        state: 'P',
        data: top !== undefined ? top : null,
        elapsed_ms: Date.now() - start,
        task_id: 0,
      };
    } catch (err) {
      return {
        state: 'T',
        data: { error: err instanceof Error ? err.message : String(err) },
        elapsed_ms: Date.now() - start,
        task_id: 0,
      };
    }
  }
}

// ═══════════════════════════════════════════════
// Export
// ═══════════════════════════════════════════════

export default CrownyClient;
