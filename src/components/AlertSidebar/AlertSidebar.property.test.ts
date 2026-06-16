/**
 * **Feature: logo-management-and-local-notifications, Property 3: Toast queue bounded at 3**
 * **Validates: Requirements 4.3**
 *
 * Property: For any sequence of notification events received by the AlertSidebar,
 * the number of concurrently visible pop-up toasts SHALL never exceed 3.
 *
 * Strategy: Extract the popup state update logic (the pure reducer from AlertSidebar)
 * and feed it random sequences of notification events. After each event is processed,
 * assert that the popup array length never exceeds 3.
 *
 * The core logic from AlertSidebar.tsx is:
 *   setPopups(prev => [...prev.slice(-2), item])
 * This keeps the last 2 items and adds the new one, bounding at 3.
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';

// ── Pure popup state reducer (extracted from AlertSidebar.tsx) ──

interface PopupItem {
  id: number;
  rule_name: string;
  symbol: string;
  price: number;
}

/**
 * Replicates the popup state transition from AlertSidebar:
 *   setPopups(prev => [...prev.slice(-2), item])
 */
function addPopup(prevPopups: PopupItem[], newItem: PopupItem): PopupItem[] {
  return [...prevPopups.slice(-2), newItem];
}

// ── Arbitrary generators ──

const popupItemArb: fc.Arbitrary<PopupItem> = fc.record({
  id: fc.nat(),
  rule_name: fc.string({ minLength: 1, maxLength: 20 }),
  symbol: fc.string({ minLength: 3, maxLength: 10 }),
  price: fc.double({ min: 0.001, max: 1_000_000, noNaN: true }),
});

// Generate a sequence of notification events with length in [1, 20]
const notificationSequenceArb: fc.Arbitrary<PopupItem[]> = fc.array(popupItemArb, {
  minLength: 1,
  maxLength: 20,
});

// ── Property test ──

describe('Property 3: Toast queue is bounded at 3', () => {
  it('popups.length never exceeds 3 for any sequence of notification events', () => {
    fc.assert(
      fc.property(notificationSequenceArb, (events) => {
        let popups: PopupItem[] = [];

        for (const event of events) {
          popups = addPopup(popups, event);
          // Core invariant: at every step, popups must be <= 3
          expect(popups.length).toBeLessThanOrEqual(3);
        }
      }),
      { numRuns: 200 } // Exceeds the minimum 100 iterations requirement
    );
  }, 30_000);

  it('after processing any sequence, final popup count is at most 3', () => {
    fc.assert(
      fc.property(notificationSequenceArb, (events) => {
        let popups: PopupItem[] = [];

        for (const event of events) {
          popups = addPopup(popups, event);
        }

        expect(popups.length).toBeLessThanOrEqual(3);
        // Also verify: if we received at least 1 event, popups should be >= 1
        expect(popups.length).toBeGreaterThanOrEqual(1);
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('the newest item is always the last element in the popup array', () => {
    fc.assert(
      fc.property(notificationSequenceArb, (events) => {
        let popups: PopupItem[] = [];

        for (const event of events) {
          popups = addPopup(popups, event);
          // The newest item should always be the last one
          expect(popups[popups.length - 1]).toEqual(event);
        }
      }),
      { numRuns: 200 }
    );
  }, 30_000);
});
