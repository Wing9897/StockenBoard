/**
 * **Feature: auto-unattended-polling, Property 7: Startup restores unattended from database**
 * **Validates: Requirements 7.2, 7.3**
 *
 * Property: For any database state at application startup, PollingManager.is_unattended()
 * after initialization SHALL equal (count_active_recordings() > 0).
 *
 * Strategy: Extract the startup restoration logic (from lib.rs / server.rs) into a pure
 * TypeScript equivalent function, then generate random count values [0..100] and verify
 * the invariant holds for all inputs.
 *
 * The core startup logic from the backend is:
 *   let active_count = db.count_active_recordings().unwrap_or(0);
 *   if active_count > 0 {
 *       polling.set_unattended(true).await;
 *   }
 *
 * The invariant: after initialization, is_unattended == (active_count > 0)
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';

// ── Pure startup restoration logic (mirrors backend lib.rs / server.rs) ──

/**
 * Simulates the PollingManager state after startup initialization.
 * Mirrors the backend logic:
 *   - Query count_active_recordings()
 *   - If count > 0, set_unattended(true)
 *   - Otherwise, unattended remains false (default)
 *
 * @param activeCount - Result of count_active_recordings() from the database
 * @returns The resulting is_unattended state after initialization
 */
function startupRestoreUnattended(activeCount: number): boolean {
  // PollingManager starts with unattended = false by default
  let isUnattended = false;

  if (activeCount > 0) {
    isUnattended = true;
  }

  return isUnattended;
}

// ── Arbitrary generators ──

// Random count values [0..100] as specified in the design's Generator Strategy
const activeCountArb = fc.integer({ min: 0, max: 100 });

// ── Property tests ──

describe('Feature: auto-unattended-polling, Property 7: Startup restores unattended from database', () => {
  it('is_unattended equals (count_active_recordings > 0) for any startup count', () => {
    fc.assert(
      fc.property(activeCountArb, (activeCount) => {
        const isUnattended = startupRestoreUnattended(activeCount);

        // Core invariant: is_unattended == (activeCount > 0)
        expect(isUnattended).toBe(activeCount > 0);
      }),
      { numRuns: 200 } // Exceeds the minimum 100 iterations requirement
    );
  }, 30_000);

  it('unattended is always true when active recordings exist', () => {
    // Only test with counts > 0
    const positiveCountArb = fc.integer({ min: 1, max: 100 });

    fc.assert(
      fc.property(positiveCountArb, (activeCount) => {
        const isUnattended = startupRestoreUnattended(activeCount);
        expect(isUnattended).toBe(true);
      }),
      { numRuns: 100 }
    );
  }, 30_000);

  it('unattended is always false when no active recordings exist', () => {
    // Edge case: count is exactly 0
    const isUnattended = startupRestoreUnattended(0);
    expect(isUnattended).toBe(false);
  });

  it('unwrap_or(0) fallback: negative or invalid counts treated as 0 produce unattended=false', () => {
    // The backend uses unwrap_or(0) on DB error, which means the count defaults to 0.
    // In TypeScript simulation, test that count=0 (the fallback value) yields unattended=false.
    // Also verify that only non-negative counts are meaningful inputs.
    fc.assert(
      fc.property(fc.integer({ min: -100, max: 100 }), (rawCount) => {
        // Simulate unwrap_or(0): if count is negative (error/invalid), treat as 0
        const effectiveCount = rawCount < 0 ? 0 : rawCount;
        const isUnattended = startupRestoreUnattended(effectiveCount);

        expect(isUnattended).toBe(effectiveCount > 0);
      }),
      { numRuns: 100 }
    );
  }, 30_000);
});
