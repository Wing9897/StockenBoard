import { describe, it, expect } from 'vitest';
import {
  formatNumber,
  formatPrice,
  truncateAddr,
  parsePairFromName,
  summarizeError,
} from './format';
import { t } from './i18n';

// Characterization tests — 捕捉這些共用純函式的現有行為，作為回歸防線。
// 這些函式被 AssetCard / DexCard 等高頻渲染元件共用，邊界分支多但無測試覆蓋。

describe('formatNumber', () => {
  it('returns "-" for null/undefined', () => {
    expect(formatNumber(null)).toBe('-');
    expect(formatNumber(undefined)).toBe('-');
  });

  it('scales large numbers with T/B/M/K suffixes', () => {
    expect(formatNumber(2.5e12)).toBe('2.50T');
    expect(formatNumber(3e9)).toBe('3.00B');
    expect(formatNumber(1.2e6)).toBe('1.20M');
    expect(formatNumber(1500)).toBe('1.50K');
  });

  it('uses the boundary suffix exactly at each threshold', () => {
    expect(formatNumber(1e12)).toBe('1.00T');
    expect(formatNumber(1e9)).toBe('1.00B');
    expect(formatNumber(1e6)).toBe('1.00M');
    expect(formatNumber(1e3)).toBe('1.00K');
  });

  it('formats sub-thousand numbers with the given decimals (default 2)', () => {
    expect(formatNumber(42)).toBe('42.00');
    expect(formatNumber(42, 0)).toBe('42');
    expect(formatNumber(3.14159, 3)).toBe('3.142');
    expect(formatNumber(0)).toBe('0.00');
  });

  it('does not scale negative numbers (they fall through the >= checks)', () => {
    expect(formatNumber(-5000)).toBe('-5000.00');
  });
});

describe('formatPrice', () => {
  it('returns "-" for null/undefined/NaN', () => {
    expect(formatPrice(null)).toBe('-');
    expect(formatPrice(undefined)).toBe('-');
    expect(formatPrice(NaN)).toBe('-');
  });

  it('prefixes USD and USDT with "$"', () => {
    expect(formatPrice(12.5)).toBe('$12.50');
    expect(formatPrice(12.5, 'USDT')).toBe('$12.50');
  });

  it('prefixes other currencies with the code and a space', () => {
    expect(formatPrice(12.5, 'EUR')).toBe('EUR 12.50');
  });

  it('uses toPrecision(4) for prices below 1', () => {
    expect(formatPrice(0.5)).toBe('$0.5000');
    expect(formatPrice(0.001234)).toBe('$0.001234');
  });

  it('uses 2 fixed decimals for prices >= 1', () => {
    expect(formatPrice(1)).toBe('$1.00');
    expect(formatPrice(99.9)).toBe('$99.90');
  });
});

describe('truncateAddr', () => {
  it('returns "-" for empty input', () => {
    expect(truncateAddr('')).toBe('-');
  });

  it('returns the address unchanged when short enough', () => {
    // default len=6 → threshold is len*2+2 = 14 chars
    expect(truncateAddr('0x1234567890')).toBe('0x1234567890'); // 12 chars
  });

  it('truncates long addresses to head...tail', () => {
    expect(truncateAddr('0x1234567890abcdef')).toBe('0x1234...cdef');
  });

  it('respects a custom head length', () => {
    expect(truncateAddr('0x1234567890abcdef', 4)).toBe('0x12...cdef');
  });
});

describe('parsePairFromName', () => {
  it('splits on a slash separator', () => {
    expect(parsePairFromName('SOL/USDC')).toEqual(['SOL', 'USDC']);
  });

  it('splits on an arrow separator and trims whitespace', () => {
    expect(parsePairFromName('SOL → USDC')).toEqual(['SOL', 'USDC']);
  });

  it('returns empty pair for names without a separator', () => {
    expect(parsePairFromName('SOLUSDC')).toEqual(['', '']);
  });

  it('returns empty pair for undefined / empty input', () => {
    expect(parsePairFromName(undefined)).toEqual(['', '']);
    expect(parsePairFromName('')).toEqual(['', '']);
  });

  it('returns empty pair when one side is missing', () => {
    expect(parsePairFromName('SOL/')).toEqual(['', '']);
    expect(parsePairFromName('/USDC')).toEqual(['', '']);
  });
});

describe('summarizeError', () => {
  it('extracts the detail from a deduped batch-failure message', () => {
    expect(summarizeError('批量查詢全部失敗 (3個): rate limited')).toBe('rate limited');
  });

  it('extracts the detail from a non-deduped batch-failure message, stopping before ": error"', () => {
    expect(summarizeError('批量查詢全部失敗: binance: rate limited: error')).toBe('rate limited');
  });

  it('maps a connection error to the i18n connectionFailed string', () => {
    expect(summarizeError('error sending request for url')).toBe(t.errors.connectionFailed);
  });

  it('truncates very long messages to 57 chars + ellipsis', () => {
    const long = 'x'.repeat(80);
    const result = summarizeError(long);
    expect(result).toBe('x'.repeat(57) + '...');
    expect(result.length).toBe(60);
  });

  it('returns short messages unchanged', () => {
    expect(summarizeError('boom')).toBe('boom');
  });
});
