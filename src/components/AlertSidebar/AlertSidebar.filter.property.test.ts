/**
 * Property-based tests for AlertSidebar filterAlerts function.
 *
 * Feature: notification-ui-refactor
 * Test file for Properties 3–6 covering text search, condition type,
 * combined intersection, and new notification visibility.
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import { filterAlerts, type AlertItem } from './AlertSidebar';

// ── Arbitrary generators ──

const CONDITION_TYPES = ['price_above', 'price_below', 'change_pct_above', 'change_pct_below', 'ai'] as const;

const conditionTypeArb: fc.Arbitrary<string> = fc.constantFrom(...CONDITION_TYPES);

const alertItemArb: fc.Arbitrary<AlertItem> = fc.record({
  id: fc.nat(),
  rule_name: fc.string({ minLength: 1, maxLength: 30 }),
  symbol: fc.string({ minLength: 1, maxLength: 10 }),
  provider: fc.string({ minLength: 1, maxLength: 15 }),
  price: fc.double({ min: 0.001, max: 1_000_000, noNaN: true }),
  condition_type: conditionTypeArb,
  threshold: fc.double({ min: 0, max: 1_000_000, noNaN: true }),
  triggered_at: fc.nat(),
  is_ai: fc.boolean(),
  ai_reason: fc.oneof(fc.constant(null), fc.string({ minLength: 1, maxLength: 50 })),
});

const alertItemListArb: fc.Arbitrary<AlertItem[]> = fc.array(alertItemArb, {
  minLength: 0,
  maxLength: 15,
});

// Filter condition type includes 'all' plus the specific types
const filterConditionTypeArb: fc.Arbitrary<string> = fc.constantFrom('all', ...CONDITION_TYPES);

// Generate non-empty search strings (for meaningful text search testing)
const searchTextArb: fc.Arbitrary<string> = fc.string({ minLength: 0, maxLength: 15 });

// ── Helper: determine if a single item satisfies both filter criteria ──

function itemSatisfiesBothCriteria(
  item: AlertItem,
  searchText: string,
  conditionType: string
): boolean {
  // Text search criterion
  if (searchText) {
    const lower = searchText.toLowerCase();
    const matchesText =
      item.rule_name.toLowerCase().includes(lower) ||
      item.symbol.toLowerCase().includes(lower) ||
      (item.ai_reason?.toLowerCase().includes(lower) ?? false);
    if (!matchesText) return false;
  }
  // Condition type criterion
  if (conditionType !== 'all' && item.condition_type !== conditionType) {
    return false;
  }
  return true;
}

// ── Property 6: New notification visibility respects active filters ──

describe('Feature: notification-ui-refactor, Property 6: New notification visibility respects active filters', () => {
  /**
   * **Validates: Requirements 4.9**
   *
   * Property: For any existing list, any active filter state, and any new AlertItem,
   * adding the item to the list and re-filtering includes it if and only if it
   * satisfies both filter criteria (text search + condition type).
   */
  it('adding a new item and re-filtering includes it iff it satisfies both filter criteria', () => {
    fc.assert(
      fc.property(
        alertItemListArb,
        searchTextArb,
        filterConditionTypeArb,
        alertItemArb,
        (existingItems, search, conditionType, newItem) => {
          // Add the new item to the list
          const updatedList = [...existingItems, newItem];

          // Compute filtered result
          const filtered = filterAlerts(updatedList, search, conditionType);

          // Determine if the new item satisfies both criteria
          const shouldBeVisible = itemSatisfiesBothCriteria(newItem, search, conditionType);

          // Check if the new item is in the filtered result (by reference identity)
          const isInFiltered = filtered.includes(newItem);

          // Assert: new item is in filtered iff it satisfies both criteria
          expect(isInFiltered).toBe(shouldBeVisible);
        }
      ),
      { numRuns: 200 }
    );
  }, 30_000);

  it('new item visibility is independent of existing list contents', () => {
    fc.assert(
      fc.property(
        alertItemListArb,
        alertItemListArb,
        searchTextArb,
        filterConditionTypeArb,
        alertItemArb,
        (listA, listB, search, conditionType, newItem) => {
          // Same new item with two different existing lists should have same visibility
          const filteredA = filterAlerts([...listA, newItem], search, conditionType);
          const filteredB = filterAlerts([...listB, newItem], search, conditionType);

          const inA = filteredA.includes(newItem);
          const inB = filteredB.includes(newItem);

          // The new item's inclusion depends only on its own properties vs filters
          expect(inA).toBe(inB);
        }
      ),
      { numRuns: 200 }
    );
  }, 30_000);
});


// ── Property 4: Condition type filter correctness ──

/**
 * **Feature: notification-ui-refactor, Property 4: Condition type filter correctness**
 * **Validates: Requirements 4.5**
 *
 * Property: For any list of AlertItems and any selected condition type (other than "all"),
 * every item in the filtered result should have a `condition_type` equal to the selected
 * filter value.
 */
describe('Feature: notification-ui-refactor, Property 4: Condition type filter correctness', () => {
  it('every item in filtered result has condition_type matching the selected filter', () => {
    fc.assert(
      fc.property(alertItemListArb, conditionTypeArb, (items, conditionType) => {
        const result = filterAlerts(items, '', conditionType);

        for (const item of result) {
          expect(item.condition_type).toBe(conditionType);
        }
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('no item excluded from result should have the matching condition_type', () => {
    fc.assert(
      fc.property(alertItemListArb, conditionTypeArb, (items, conditionType) => {
        const result = filterAlerts(items, '', conditionType);
        const resultIds = new Set(result.map(item => item.id));
        const excluded = items.filter(item => !resultIds.has(item.id));

        for (const item of excluded) {
          expect(item.condition_type).not.toBe(conditionType);
        }
      }),
      { numRuns: 200 }
    );
  }, 30_000);

  it('result is a subset of the original items', () => {
    fc.assert(
      fc.property(alertItemListArb, conditionTypeArb, (items, conditionType) => {
        const result = filterAlerts(items, '', conditionType);

        expect(result.length).toBeLessThanOrEqual(items.length);
        for (const item of result) {
          expect(items).toContain(item);
        }
      }),
      { numRuns: 200 }
    );
  }, 30_000);
});
