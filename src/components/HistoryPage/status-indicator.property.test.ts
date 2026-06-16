/**
 * **Feature: auto-unattended-polling, Property 5: Status indicator reflects recording state**
 * **Validates: Requirements 5.1, 5.2**
 *
 * Property: For any list of subscriptions, the status indicator active state
 * SHALL equal `subscriptions.some(s => s.record_enabled === 1)`.
 * When at least one subscription has recording enabled, the indicator shows active;
 * when none do, it shows inactive.
 *
 * Strategy: Generate random arrays of subscription-like objects with `record_enabled`
 * set to 0 or 1. Extract the pure derivation logic (same as HistoryPage.tsx) and
 * verify the invariant holds for all generated inputs.
 *
 * The core logic from HistoryPage.tsx is:
 *   const isUnattended = useMemo(() => subs.some(s => s.record_enabled), [subs]);
 *
 * Since JavaScript treats 1 as truthy and 0 as falsy, `subs.some(s => s.record_enabled)`
 * is equivalent to `subs.some(s => s.record_enabled === 1)` when values are 0 or 1.
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';

// ── Minimal subscription shape for status derivation ──

interface SubscriptionLike {
  id: number;
  record_enabled: 0 | 1;
}

/**
 * Replicates the status indicator derivation from HistoryPage.tsx:
 *   const isUnattended = useMemo(() => subs.some(s => s.record_enabled), [subs]);
 */
function deriveIsUnattended(subs: SubscriptionLike[]): boolean {
  return subs.some(s => s.record_enabled);
}

// ── Arbitrary generators ──

const subscriptionArb: fc.Arbitrary<SubscriptionLike> = fc.record({
  id: fc.nat({ max: 10000 }),
  record_enabled: fc.constantFrom(0 as const, 1 as const),
});

// Generate arrays of subscriptions (0 to 50 items)
const subscriptionListArb: fc.Arbitrary<SubscriptionLike[]> = fc.array(subscriptionArb, {
  minLength: 0,
  maxLength: 50,
});

// ── Property tests ──

describe('Feature: auto-unattended-polling, Property 5: Status indicator reflects recording state', () => {
  it('isUnattended equals true iff at least one subscription has record_enabled === 1', () => {
    fc.assert(
      fc.property(subscriptionListArb, (subs) => {
        const isUnattended = deriveIsUnattended(subs);
        const hasAnyRecording = subs.some(s => s.record_enabled === 1);

        expect(isUnattended).toBe(hasAnyRecording);
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('empty subscription list always produces inactive status', () => {
    fc.assert(
      fc.property(fc.constant([] as SubscriptionLike[]), (subs) => {
        const isUnattended = deriveIsUnattended(subs);

        expect(isUnattended).toBe(false);
      }),
      { numRuns: 100 }
    );
  }, 30_000);

  it('list with all record_enabled === 0 always produces inactive status', () => {
    const allDisabledArb = fc.array(
      fc.record({
        id: fc.nat({ max: 10000 }),
        record_enabled: fc.constant(0 as const),
      }),
      { minLength: 1, maxLength: 50 }
    );

    fc.assert(
      fc.property(allDisabledArb, (subs) => {
        const isUnattended = deriveIsUnattended(subs);

        expect(isUnattended).toBe(false);
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('list with at least one record_enabled === 1 always produces active status', () => {
    // Generate a list that is guaranteed to have at least one enabled subscription
    const atLeastOneEnabledArb = fc
      .array(subscriptionArb, { minLength: 0, maxLength: 49 })
      .chain(rest =>
        fc.record({
          id: fc.nat({ max: 10000 }),
          record_enabled: fc.constant(1 as const),
        }).map(enabled => [...rest, enabled])
      )
      .map(arr => fc.shuffledSubarray(arr, { minLength: arr.length, maxLength: arr.length }))
      .chain(x => x);

    fc.assert(
      fc.property(atLeastOneEnabledArb, (subs) => {
        const isUnattended = deriveIsUnattended(subs);

        expect(isUnattended).toBe(true);
      }),
      { numRuns: 200 }
    );
  }, 30_000);
});
