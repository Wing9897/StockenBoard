/**
 * **Feature: ai-rule-enhancements, Property 6: subscription_id backward-compatibility invariant**
 * **Validates: Requirements 5.3**
 *
 * Property: For any non-empty JSON array of subscription IDs stored in `subscription_ids`,
 * the `subscription_id` column SHALL equal the first element of that array.
 *
 * Strategy: Use fast-check to generate random non-empty arrays of subscription IDs (i64-range
 * integers) and verify that `deriveSubscriptionId(ids)` always returns `ids[0]`, and that
 * `buildSubscriptionPayload(ids)` always produces a payload where subscription_id === ids[0].
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import { deriveSubscriptionId, buildSubscriptionPayload } from './subscriptionCompat';

// ── Arbitrary generators ──

// Subscription IDs are i64 in the database; we use safe integer range
const subscriptionIdArb = fc.integer({ min: 1, max: Number.MAX_SAFE_INTEGER });

// Non-empty arrays of subscription IDs (1 to 20 elements)
const nonEmptySubscriptionIdsArb = fc.array(subscriptionIdArb, { minLength: 1, maxLength: 20 });

// ── Property tests ──

describe('Property 6: subscription_id backward-compatibility invariant', () => {
  it('deriveSubscriptionId(ids) === ids[0] for any non-empty subscription_ids array', () => {
    fc.assert(
      fc.property(nonEmptySubscriptionIdsArb, (ids) => {
        const result = deriveSubscriptionId(ids);
        expect(result).toBe(ids[0]);
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('buildSubscriptionPayload(ids).subscription_id === ids[0] for any non-empty array', () => {
    fc.assert(
      fc.property(nonEmptySubscriptionIdsArb, (ids) => {
        const payload = buildSubscriptionPayload(ids);
        expect(payload.subscription_id).toBe(ids[0]);
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('buildSubscriptionPayload preserves the full subscription_ids array unchanged', () => {
    fc.assert(
      fc.property(nonEmptySubscriptionIdsArb, (ids) => {
        const payload = buildSubscriptionPayload(ids);
        expect(payload.subscription_ids).toEqual(ids);
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('invariant holds for single-element arrays (most common pre-migration case)', () => {
    fc.assert(
      fc.property(subscriptionIdArb, (id) => {
        const ids = [id];
        const result = deriveSubscriptionId(ids);
        expect(result).toBe(id);
        const payload = buildSubscriptionPayload(ids);
        expect(payload.subscription_id).toBe(id);
        expect(payload.subscription_ids).toEqual([id]);
      }),
      { numRuns: 200 }
    );
  }, 30_000);
});
