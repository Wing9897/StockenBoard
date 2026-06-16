/**
 * Feature: auto-unattended-polling, Property 1–4: toggle_record logic
 *
 * These property-based tests verify the core toggle_record command logic
 * for arbitrary subscription IDs and database states.
 *
 * The toggle_record logic (from the Rust backend) is modeled as a pure function
 * here for testability:
 * - Enable + count==0 + no confirmed → { success: false, needs_confirm: true }
 * - Enable + count==0 + confirmed=true → enables recording, sets unattended true
 * - Enable + count>0 → enables recording directly
 * - Disable → disables recording, checks remaining count, if 0 sets unattended false
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';

// ── Model of backend state and toggle_record logic ──

interface ToggleRecordResponse {
  success: boolean;
  needs_confirm: boolean;
}

interface DbState {
  /** Number of currently active recordings (before this toggle call) */
  activeCount: number;
  /** Whether the target subscription currently has recording enabled */
  targetRecordEnabled: boolean;
}

interface PollingState {
  unattended: boolean;
}

interface ToggleResult {
  response: ToggleRecordResponse;
  /** Whether the target subscription's record_enabled was changed */
  recordEnabledAfter: boolean;
  /** The polling unattended state after the operation */
  pollingState: PollingState;
  /** The remaining active count after the operation (for disable path) */
  remainingCount: number;
}

/**
 * Pure model of the toggle_record command logic, matching the Rust implementation.
 */
function toggleRecord(
  subscriptionId: number,
  enabled: boolean,
  confirmed: boolean | undefined,
  db: DbState,
  polling: PollingState,
): ToggleResult {
  const pollingAfter = { ...polling };
  let recordEnabledAfter = db.targetRecordEnabled;
  let remainingCount = db.activeCount;

  if (enabled) {
    const activeCount = db.activeCount;
    if (activeCount === 0 && confirmed !== true) {
      // First recording: require confirmation
      return {
        response: { success: false, needs_confirm: true },
        recordEnabledAfter,
        pollingState: pollingAfter,
        remainingCount,
      };
    }
    // Enable recording
    recordEnabledAfter = true;
    remainingCount = activeCount + (db.targetRecordEnabled ? 0 : 1);
    // Enable unattended if transitioning from 0
    if (activeCount === 0) {
      pollingAfter.unattended = true;
    }
  } else {
    // Disable recording
    recordEnabledAfter = false;
    // Remaining count = activeCount minus 1 (the one we just disabled)
    remainingCount = db.targetRecordEnabled ? db.activeCount - 1 : db.activeCount;
    if (remainingCount === 0) {
      pollingAfter.unattended = false;
    }
  }

  return {
    response: { success: true, needs_confirm: false },
    recordEnabledAfter,
    pollingState: pollingAfter,
    remainingCount,
  };
}

// ── Arbitrary generators ──

/** Arbitrary subscription ID (positive integer) */
const subscriptionIdArb = fc.integer({ min: 1, max: 1_000_000 });

// ── Property Tests ──

describe('Feature: auto-unattended-polling, Property 1: Enable requires confirmation when no active recordings', () => {
  it('toggle_record(id, true) without confirmed returns needs_confirm when count==0', () => {
    fc.assert(
      fc.property(
        subscriptionIdArb,
        fc.boolean(), // initial unattended state (should be false when count=0, but test either)
        (subId, initialUnattended) => {
          const db: DbState = { activeCount: 0, targetRecordEnabled: false };
          const polling: PollingState = { unattended: initialUnattended };

          // Call without confirmed (undefined)
          const result = toggleRecord(subId, true, undefined, db, polling);

          // SHALL return needs_confirm: true, success: false
          expect(result.response).toEqual({ success: false, needs_confirm: true });
          // SHALL NOT modify the subscription's record_enabled field
          expect(result.recordEnabledAfter).toBe(false);
          // Polling state should not change
          expect(result.pollingState.unattended).toBe(initialUnattended);
        },
      ),
      { numRuns: 100 },
    );
  }, 30_000);

  it('toggle_record(id, true, confirmed=false) also returns needs_confirm when count==0', () => {
    fc.assert(
      fc.property(subscriptionIdArb, (subId) => {
        const db: DbState = { activeCount: 0, targetRecordEnabled: false };
        const polling: PollingState = { unattended: false };

        // confirmed=false should behave like undefined (not Some(true))
        const result = toggleRecord(subId, true, false, db, polling);

        expect(result.response).toEqual({ success: false, needs_confirm: true });
        expect(result.recordEnabledAfter).toBe(false);
      }),
      { numRuns: 100 },
    );
  }, 30_000);
});

/**
 * **Validates: Requirements 2.2**
 */
describe('Feature: auto-unattended-polling, Property 2: Enable succeeds directly when recordings already active', () => {
  it('toggle_record(id, true) succeeds without confirmation when count>0', () => {
    fc.assert(
      fc.property(
        subscriptionIdArb,
        fc.integer({ min: 1, max: 100 }), // active count > 0
        (subId, activeCount) => {
          const db: DbState = { activeCount, targetRecordEnabled: false };
          const polling: PollingState = { unattended: true };

          const result = toggleRecord(subId, true, undefined, db, polling);

          // SHALL return success: true, needs_confirm: false
          expect(result.response).toEqual({ success: true, needs_confirm: false });
          // SHALL set record_enabled to true
          expect(result.recordEnabledAfter).toBe(true);
          // Unattended should remain true (already active)
          expect(result.pollingState.unattended).toBe(true);
        },
      ),
      { numRuns: 100 },
    );
  }, 30_000);

  it('enable succeeds regardless of confirmed flag when count>0', () => {
    fc.assert(
      fc.property(
        subscriptionIdArb,
        fc.integer({ min: 1, max: 100 }),
        fc.option(fc.boolean()), // confirmed can be undefined, true, or false
        (subId, activeCount, confirmed) => {
          const db: DbState = { activeCount, targetRecordEnabled: false };
          const polling: PollingState = { unattended: true };

          const result = toggleRecord(
            subId,
            true,
            confirmed === null ? undefined : confirmed,
            db,
            polling,
          );

          // Always succeeds when count > 0
          expect(result.response.success).toBe(true);
          expect(result.response.needs_confirm).toBe(false);
          expect(result.recordEnabledAfter).toBe(true);
        },
      ),
      { numRuns: 100 },
    );
  }, 30_000);
});

/**
 * **Validates: Requirements 2.3**
 */
describe('Feature: auto-unattended-polling, Property 3: Confirmed enable activates recording and unattended', () => {
  it('toggle_record(id, true, confirmed=true) enables recording and sets unattended when count==0', () => {
    fc.assert(
      fc.property(subscriptionIdArb, (subId) => {
        const db: DbState = { activeCount: 0, targetRecordEnabled: false };
        const polling: PollingState = { unattended: false };

        const result = toggleRecord(subId, true, true, db, polling);

        // SHALL set record_enabled to true
        expect(result.recordEnabledAfter).toBe(true);
        // SHALL set unattended to true
        expect(result.pollingState.unattended).toBe(true);
        // SHALL return success
        expect(result.response).toEqual({ success: true, needs_confirm: false });
      }),
      { numRuns: 100 },
    );
  }, 30_000);

  it('confirmed enable from zero count always transitions unattended from false to true', () => {
    fc.assert(
      fc.property(
        subscriptionIdArb,
        fc.boolean(), // any initial unattended state
        (subId, initialUnattended) => {
          const db: DbState = { activeCount: 0, targetRecordEnabled: false };
          const polling: PollingState = { unattended: initialUnattended };

          const result = toggleRecord(subId, true, true, db, polling);

          // Unattended MUST be true after confirmed enable from 0
          expect(result.pollingState.unattended).toBe(true);
          expect(result.recordEnabledAfter).toBe(true);
          expect(result.response.success).toBe(true);
        },
      ),
      { numRuns: 100 },
    );
  }, 30_000);
});

/**
 * **Validates: Requirements 3.1, 3.2, 6.3**
 */
describe('Feature: auto-unattended-polling, Property 4: Disable maintains unattended invariant', () => {
  it('after disable, unattended == (remainingCount > 0)', () => {
    fc.assert(
      fc.property(
        subscriptionIdArb,
        fc.integer({ min: 1, max: 50 }), // initial active count (at least 1 since target is active)
        (subId, activeCount) => {
          // The target subscription is currently recording
          const db: DbState = { activeCount, targetRecordEnabled: true };
          const polling: PollingState = { unattended: true };

          const result = toggleRecord(subId, false, undefined, db, polling);

          // After disabling, remaining = activeCount - 1
          const expectedRemaining = activeCount - 1;
          expect(result.remainingCount).toBe(expectedRemaining);

          // Core invariant: unattended == (remaining > 0)
          expect(result.pollingState.unattended).toBe(expectedRemaining > 0);
          // Record should be disabled
          expect(result.recordEnabledAfter).toBe(false);
          // Response should be success
          expect(result.response).toEqual({ success: true, needs_confirm: false });
        },
      ),
      { numRuns: 100 },
    );
  }, 30_000);

  it('disabling the last recording (count=1) sets unattended to false', () => {
    fc.assert(
      fc.property(subscriptionIdArb, (subId) => {
        // Exactly 1 active recording — the one we're about to disable
        const db: DbState = { activeCount: 1, targetRecordEnabled: true };
        const polling: PollingState = { unattended: true };

        const result = toggleRecord(subId, false, undefined, db, polling);

        // Unattended must be false after last recording disabled
        expect(result.pollingState.unattended).toBe(false);
        expect(result.remainingCount).toBe(0);
        expect(result.recordEnabledAfter).toBe(false);
      }),
      { numRuns: 100 },
    );
  }, 30_000);

  it('disabling when multiple recordings remain keeps unattended true', () => {
    fc.assert(
      fc.property(
        subscriptionIdArb,
        fc.integer({ min: 2, max: 50 }), // more than 1 active
        (subId, activeCount) => {
          const db: DbState = { activeCount, targetRecordEnabled: true };
          const polling: PollingState = { unattended: true };

          const result = toggleRecord(subId, false, undefined, db, polling);

          // Unattended must remain true since remaining > 0
          expect(result.pollingState.unattended).toBe(true);
          expect(result.remainingCount).toBe(activeCount - 1);
          expect(result.remainingCount).toBeGreaterThan(0);
        },
      ),
      { numRuns: 100 },
    );
  }, 30_000);
});
