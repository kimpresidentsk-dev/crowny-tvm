import { describe, it, expect } from 'vitest';
import { Trit, CtpHeader, CrownyLocal, CrownyClient } from './index';

describe('Trit', () => {
  it('converts to/from numbers', () => {
    expect(Trit.toNumber('P')).toBe(1);
    expect(Trit.toNumber('O')).toBe(0);
    expect(Trit.toNumber('T')).toBe(-1);
    expect(Trit.from(5)).toBe('P');
    expect(Trit.from(0)).toBe('O');
    expect(Trit.from(-3)).toBe('T');
  });

  it('converts to Korean', () => {
    expect(Trit.toKorean('P')).toBe('성공');
    expect(Trit.toKorean('O')).toBe('보류');
    expect(Trit.toKorean('T')).toBe('실패');
  });

  it('performs NOT', () => {
    expect(Trit.not('P')).toBe('T');
    expect(Trit.not('T')).toBe('P');
    expect(Trit.not('O')).toBe('O');
  });

  it('performs AND (min)', () => {
    expect(Trit.and('P', 'P')).toBe('P');
    expect(Trit.and('P', 'O')).toBe('O');
    expect(Trit.and('P', 'T')).toBe('T');
    expect(Trit.and('O', 'O')).toBe('O');
  });

  it('performs OR (max)', () => {
    expect(Trit.or('P', 'T')).toBe('P');
    expect(Trit.or('T', 'T')).toBe('T');
    expect(Trit.or('O', 'P')).toBe('P');
  });

  it('computes consensus', () => {
    expect(Trit.consensus(['P', 'P', 'P'])).toBe('P');
    expect(Trit.consensus(['T', 'T', 'T'])).toBe('T');
    expect(Trit.consensus(['P', 'P', 'T'])).toBe('P');
    expect(Trit.consensus(['P', 'T', 'T'])).toBe('T');
    expect(Trit.consensus(['P', 'O', 'T'])).toBe('O');
  });
});

describe('CtpHeader', () => {
  it('creates success header', () => {
    const h = CtpHeader.success();
    expect(h.state).toBe('P');
    expect(h.permission).toBe('P');
    expect(h.consensus).toBe('P');
    expect(h.toString().startsWith('PPP')).toBe(true);
  });

  it('parses string', () => {
    const h = CtpHeader.fromString('PPTOOOOO0');
    expect(h.trits[0]).toBe('P');
    expect(h.trits[2]).toBe('T');
  });

  it('computes overall state', () => {
    const h = CtpHeader.failed();
    expect(h.overallState).toBe('T');

    const h2 = CtpHeader.success();
    // has O trits after first 3, so overall is O
    expect(h2.overallState).toBe('O');
  });
});

describe('CrownyLocal', () => {
  it('executes simple addition', () => {
    const local = new CrownyLocal();
    const result = local.execute('넣어 10\n넣어 20\n더해\n종료');
    expect(result.state).toBe('P');
    expect(result.data).toBe(30);
  });

  it('executes multiplication', () => {
    const local = new CrownyLocal();
    const result = local.execute('넣어 7\n넣어 6\n곱해\n종료');
    expect(result.state).toBe('P');
    expect(result.data).toBe(42);
  });

  it('executes subtraction', () => {
    const local = new CrownyLocal();
    const result = local.execute('넣어 100\n넣어 30\n빼\n종료');
    expect(result.state).toBe('P');
    expect(result.data).toBe(70);
  });

  it('handles trit literals', () => {
    const local = new CrownyLocal();
    const result = local.execute('참\n종료');
    expect(result.state).toBe('P');
    expect(result.data).toBe(1);
  });

  it('handles string push', () => {
    const local = new CrownyLocal();
    const result = local.execute('넣어 "hello"\n종료');
    expect(result.state).toBe('P');
    expect(result.data).toBe('hello');
  });

  it('handles English syntax', () => {
    const local = new CrownyLocal();
    const result = local.execute('push 5\npush 3\nadd\nend');
    expect(result.state).toBe('P');
    expect(result.data).toBe(8);
  });

  it('handles empty program', () => {
    const local = new CrownyLocal();
    const result = local.execute('종료');
    expect(result.state).toBe('P');
    expect(result.data).toBe(null);
  });
});

describe('CrownyClient', () => {
  it('creates with string URL', () => {
    const client = new CrownyClient('http://localhost:7293');
    expect(client.getStats().total).toBe(0);
  });

  it('creates with config object', () => {
    const client = new CrownyClient({ baseUrl: 'http://localhost:7293', timeout: 5000 });
    expect(client.getHistory()).toEqual([]);
  });
});
