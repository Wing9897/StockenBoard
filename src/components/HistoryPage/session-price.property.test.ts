/**
 * **Feature: recording-ui-optimization, Property 1: Session-aware price selection with fallback**
 * **Validates: Requirements 1.5, 1.6**
 *
 * Property: For any PriceHistoryRecord and any SessionFilter value, the `getPrice`
 * function SHALL return the session-specific price when it is non-null (pre_price
 * for 'pre', post_price for 'post'), and SHALL fall back to the regular `price`
 * field when the session-specific price is null or the session is 'regular'.
 *
 * Strategy: Generate arbitrary PriceHistoryRecord objects with nullable pre_price
 * and post_price fields, paired with a random SessionFilter. Verify the selection
 * logic matches the expected behavior for all combinations.
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';

// ── Types ──

type SessionFilter = 'regular' | 'pre' | 'post';

interface PriceHistoryRecord {
  id: number;
  subscription_id: number;
  provider_id: string;
  price: number;
  change_pct: number | null;
  volume: number | null;
  pre_price: number | null;
  post_price: number | null;
  recorded_at: number;
}

// ── Pure function under test (replicated from HistoryChart.tsx) ──

function getPrice(record: PriceHistoryRecord, session: SessionFilter): number {
  if (session === 'pre' && record.pre_price != null) return record.pre_price;
  if (session === 'post' && record.post_price != null) return record.post_price;
  return record.price;
}

// ── Arbitrary generators ──

const nullableNumber: fc.Arbitrary<number | null> = fc.oneof(
  fc.constant(null),
  fc.double({ min: -10000, max: 10000, noNaN: true, noDefaultInfinity: true }),
);

const priceRecordArb: fc.Arbitrary<PriceHistoryRecord> = fc.record({
  id: fc.nat({ max: 100000 }),
  subscription_id: fc.nat({ max: 10000 }),
  provider_id: fc.string({ minLength: 1, maxLength: 20 }),
  price: fc.double({ min: 0.01, max: 10000, noNaN: true, noDefaultInfinity: true }),
  change_pct: nullableNumber,
  volume: fc.oneof(fc.constant(null), fc.nat({ max: 1000000000 })),
  pre_price: nullableNumber,
  post_price: nullableNumber,
  recorded_at: fc.nat({ max: 2000000000 }),
});

const sessionFilterArb: fc.Arbitrary<SessionFilter> = fc.constantFrom(
  'regular' as const,
  'pre' as const,
  'post' as const,
);

// ── Property tests ──

describe('Feature: recording-ui-optimization, Property 1: Session-aware price selection with fallback', () => {
  it('returns correct price for any record and session combination', () => {
    fc.assert(
      fc.property(priceRecordArb, sessionFilterArb, (record, session) => {
        const result = getPrice(record, session);

        if (session === 'pre' && record.pre_price != null) {
          expect(result).toBe(record.pre_price);
        } else if (session === 'post' && record.post_price != null) {
          expect(result).toBe(record.post_price);
        } else {
          expect(result).toBe(record.price);
        }
      }),
      { numRuns: 100 },
    );
  }, 30_000);

  it('always returns the regular price when session is "regular", regardless of pre/post prices', () => {
    fc.assert(
      fc.property(priceRecordArb, (record) => {
        const result = getPrice(record, 'regular');
        expect(result).toBe(record.price);
      }),
      { numRuns: 100 },
    );
  }, 30_000);

  it('falls back to regular price when session-specific price is null', () => {
    // Generate records where the session-specific price is always null
    const recordWithNullSessionPrices = fc.record({
      id: fc.nat({ max: 100000 }),
      subscription_id: fc.nat({ max: 10000 }),
      provider_id: fc.string({ minLength: 1, maxLength: 20 }),
      price: fc.double({ min: 0.01, max: 10000, noNaN: true, noDefaultInfinity: true }),
      change_pct: nullableNumber,
      volume: fc.oneof(fc.constant(null), fc.nat({ max: 1000000000 })),
      pre_price: fc.constant(null),
      post_price: fc.constant(null),
      recorded_at: fc.nat({ max: 2000000000 }),
    });

    fc.assert(
      fc.property(recordWithNullSessionPrices, sessionFilterArb, (record, session) => {
        const result = getPrice(record, session);
        expect(result).toBe(record.price);
      }),
      { numRuns: 100 },
    );
  }, 30_000);
});
