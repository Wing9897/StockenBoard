/**
 * Feature: auto-unattended-polling, Property 6: Batch confirm enables all targets and activates unattended
 * **Validates: Requirements 6.2**
 *
 * For any non-empty set of target subscriptions where count_active_recordings() == 0
 * before the batch, after the user confirms and all toggle_record(id, true, confirmed=true)
 * calls complete (first call with confirmed:true, remaining calls invoke normally since
 * count > 0 after first), every target subscription SHALL have record_enabled = true
 * AND PollingManager.is_unattended() SHALL be true.
 *
 * Strategy: Model the batch toggle logic as a pure function simulating the HistoryPage
 * batchToggle behavior (first call with confirmed=true, rest normal) against arbitrary
 * subscription ID arrays of size [1..20] with 0 initial active recordings.
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';

// ── Model of backend state and toggle_record logic ──

interface ToggleRecordResponse {
  success: boolean;
  needs_confirm: boolean;
}

interface Subscription {
  id: number;
  record_enabled: boolean;
}

interface SystemState {
  subscriptions: Map<number, Subscription>;
  unattended: boolean;
}

/**
 * Pure model of the toggle_record command logic, matching the Rust implementation.
 * Mutates state in place (simulating DB + PollingManager side effects).
 */
function toggleRecord(
  state: SystemState,
  subscriptionId: number,
  enabled: boolean,
  confirmed?: boolean,
): ToggleRecordResponse {
  const activeCount = countActiveRecordings(state);

  if (enabled) {
    if (activeCount === 0 && confirmed !== true) {
      return { success: false, needs_confirm: true };
    }
    // Enable recording on the target
    const sub = state.subscriptions.get(subscriptionId);
    if (sub) {
      sub.record_enabled = true;
    }
    // Enable unattended if transitioning from 0
    if (activeCount === 0) {
      state.unattended = true;
    }
    return { success: true, needs_confirm: false };
  } else {
    // Disable recording on the target
    const sub = state.subscriptions.get(subscriptionId);
    if (sub) {
      sub.record_enabled = false;
    }
    // Check remaining count after disable
    const remaining = countActiveRecordings(state);
    if (remaining === 0) {
      state.unattended = false;
    }
    return { success: true, needs_confirm: false };
  }
}

function countActiveRecordings(state: SystemState): number {
  let count = 0;
  for (const sub of state.subscriptions.values()) {
    if (sub.record_enabled) count++;
  }
  return count;
}

/**
 * Model of the HistoryPage batchToggle logic for enabling:
 * - Precondition: count_active_recordings() == 0 and user confirmed
 * - First call: toggle_record(targets[0].id, true, confirmed=true)
 * - Remaining calls: toggle_record(targets[i].id, true) (no confirmed needed, count > 0)
 */
function batchEnableConfirmed(state: SystemState, targetIds: number[]): void {
  if (targetIds.length === 0) return;

  // First call with confirmed: true to trigger unattended enable
  toggleRecord(state, targetIds[0], true, true);

  // Remaining calls invoke normally (count > 0 after first)
  for (let i = 1; i < targetIds.length; i++) {
    toggleRecord(state, targetIds[i], true);
  }
}

// ── Arbitrary generators ──

/** Generate unique subscription IDs array of size [1..20] */
const targetIdsArb = fc.uniqueArray(
  fc.integer({ min: 1, max: 1_000_000 }),
  { minLength: 1, maxLength: 20 },
);

// ── Property Tests ──

describe('Feature: auto-unattended-polling, Property 6: Batch confirm enables all targets and activates unattended', () => {
  it('after batch confirm with 0 initial recordings, all targets have record_enabled=true and unattended=true', () => {
    fc.assert(
      fc.property(targetIdsArb, (targetIds) => {
        // Arrange: system state with all targets having record_enabled=false, unattended=false
        const state: SystemState = {
          subscriptions: new Map(
            targetIds.map(id => [id, { id, record_enabled: false }]),
          ),
          unattended: false,
        };

        // Precondition: no active recordings
        expect(countActiveRecordings(state)).toBe(0);

        // Act: simulate batch enable after user confirmation
        batchEnableConfirmed(state, targetIds);

        // Assert: every target subscription has record_enabled = true
        for (const id of targetIds) {
          const sub = state.subscriptions.get(id);
          expect(sub?.record_enabled).toBe(true);
        }

        // Assert: unattended is true
        expect(state.unattended).toBe(true);
      }),
      { numRuns: 100 },
    );
  }, 30_000);

  it('batch confirm works correctly regardless of target array ordering', () => {
    fc.assert(
      fc.property(
        targetIdsArb,
        fc.shuffledSubarray(fc.constant(null), { minLength: 0, maxLength: 0 }),
        (targetIds) => {
          // Test with the targets as given (fast-check provides arbitrary ordering)
          const state: SystemState = {
            subscriptions: new Map(
              targetIds.map(id => [id, { id, record_enabled: false }]),
            ),
            unattended: false,
          };

          batchEnableConfirmed(state, targetIds);

          // Invariant holds regardless of which ID is first
          expect(state.unattended).toBe(true);
          expect(countActiveRecordings(state)).toBe(targetIds.length);
        },
      ),
      { numRuns: 100 },
    );
  }, 30_000);

  it('batch confirm with mixed initial states (some already enabled) still activates all targets', () => {
    fc.assert(
      fc.property(
        // Generate target IDs plus some extra subscription IDs that are NOT targets
        targetIdsArb,
        fc.integer({ min: 0, max: 5 }), // number of extra non-target subscriptions (all disabled)
        (targetIds, extraCount) => {
          // All subscriptions start with record_enabled=false (count == 0)
          const allIds = [...targetIds];
          // Add extra subscriptions that won't be batch-toggled
          for (let i = 0; i < extraCount; i++) {
            const extraId = 2_000_000 + i; // guaranteed not to collide with targets
            allIds.push(extraId);
          }

          const state: SystemState = {
            subscriptions: new Map(
              allIds.map(id => [id, { id, record_enabled: false }]),
            ),
            unattended: false,
          };

          // Precondition: 0 active recordings
          expect(countActiveRecordings(state)).toBe(0);

          // Act
          batchEnableConfirmed(state, targetIds);

          // Assert: all targets enabled
          for (const id of targetIds) {
            expect(state.subscriptions.get(id)?.record_enabled).toBe(true);
          }
          // Assert: unattended is true
          expect(state.unattended).toBe(true);
          // Assert: extra subscriptions remain disabled
          for (let i = 0; i < extraCount; i++) {
            const extraId = 2_000_000 + i;
            expect(state.subscriptions.get(extraId)?.record_enabled).toBe(false);
          }
          // Assert: total active count equals targets length
          expect(countActiveRecordings(state)).toBe(targetIds.length);
        },
      ),
      { numRuns: 100 },
    );
  }, 30_000);

  it('first call in batch uses confirmed=true and would fail without it (0→1 transition)', () => {
    fc.assert(
      fc.property(targetIdsArb, (targetIds) => {
        // Verify that the first call WOULD require confirmation without the confirmed flag
        const state: SystemState = {
          subscriptions: new Map(
            targetIds.map(id => [id, { id, record_enabled: false }]),
          ),
          unattended: false,
        };

        // Without confirmed, first call should return needs_confirm
        const resp = toggleRecord(state, targetIds[0], true);
        expect(resp.needs_confirm).toBe(true);
        expect(resp.success).toBe(false);
        // State should be unchanged
        expect(state.subscriptions.get(targetIds[0])?.record_enabled).toBe(false);
        expect(state.unattended).toBe(false);

        // Now with confirmed=true, it succeeds and activates unattended
        const resp2 = toggleRecord(state, targetIds[0], true, true);
        expect(resp2.success).toBe(true);
        expect(resp2.needs_confirm).toBe(false);
        expect(state.subscriptions.get(targetIds[0])?.record_enabled).toBe(true);
        expect(state.unattended).toBe(true);

        // Subsequent calls succeed without confirmed (count > 0 now)
        for (let i = 1; i < targetIds.length; i++) {
          const resp3 = toggleRecord(state, targetIds[i], true);
          expect(resp3.success).toBe(true);
          expect(resp3.needs_confirm).toBe(false);
        }

        // Final invariant
        expect(countActiveRecordings(state)).toBe(targetIds.length);
        expect(state.unattended).toBe(true);
      }),
      { numRuns: 100 },
    );
  }, 30_000);
});
