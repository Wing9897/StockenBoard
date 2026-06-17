/**
 * **Feature: ai-rule-enhancements, Property 1: Token estimation formula correctness (frontend)**
 * **Validates: Requirements 2.1, 2.3**
 *
 * Property: For any non-negative integers `numSubscriptions` and `historyWindow`,
 * `estimateTokens(numSubscriptions, historyWindow)` SHALL equal
 * `numSubscriptions * historyWindow * 25 + 300`.
 *
 * Strategy: Use fast-check to generate random (numSubscriptions, historyWindow) pairs
 * and verify:
 * 1. The formula: estimateTokens(n, h) === n * h * TOKENS_PER_RECORD + TOTAL_OVERHEAD
 * 2. Result is always >= TOTAL_OVERHEAD (300)
 * 3. Monotonicity: increasing either input doesn't decrease the result
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import {
  estimateTokens,
  TOKENS_PER_RECORD,
  TOTAL_OVERHEAD,
} from './tokenEstimator';

// ── Arbitrary generators ──

// Non-negative integers within a reasonable range to avoid overflow
const numSubscriptionsArb = fc.nat({ max: 1000 });
const historyWindowArb = fc.nat({ max: 1000 });

// ── Property tests ──

describe('Property 1: Token estimation formula correctness (frontend)', () => {
  it('estimateTokens(n, h) === n * h * TOKENS_PER_RECORD + TOTAL_OVERHEAD for all non-negative inputs', () => {
    fc.assert(
      fc.property(numSubscriptionsArb, historyWindowArb, (n, h) => {
        const result = estimateTokens(n, h);
        const expected = n * h * TOKENS_PER_RECORD + TOTAL_OVERHEAD;
        expect(result).toBe(expected);
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('result is always >= TOTAL_OVERHEAD (300) for any non-negative inputs', () => {
    fc.assert(
      fc.property(numSubscriptionsArb, historyWindowArb, (n, h) => {
        const result = estimateTokens(n, h);
        expect(result).toBeGreaterThanOrEqual(TOTAL_OVERHEAD);
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('monotonicity: increasing numSubscriptions does not decrease the result', () => {
    fc.assert(
      fc.property(numSubscriptionsArb, numSubscriptionsArb, historyWindowArb, (n1, n2, h) => {
        const smaller = Math.min(n1, n2);
        const larger = Math.max(n1, n2);
        expect(estimateTokens(larger, h)).toBeGreaterThanOrEqual(estimateTokens(smaller, h));
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('monotonicity: increasing historyWindow does not decrease the result', () => {
    fc.assert(
      fc.property(numSubscriptionsArb, historyWindowArb, historyWindowArb, (n, h1, h2) => {
        const smaller = Math.min(h1, h2);
        const larger = Math.max(h1, h2);
        expect(estimateTokens(n, larger)).toBeGreaterThanOrEqual(estimateTokens(n, smaller));
      }),
      { numRuns: 200 }
    );
  }, 30_000);
});
