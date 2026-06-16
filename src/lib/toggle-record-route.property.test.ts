/**
 * Feature: recording-ui-optimization, Property 2: toggle_record HTTP route mapping correctness
 *
 * **Validates: Requirements 3.1, 3.2, 3.5**
 *
 * For any positive integer subscription ID and any combination of enabled (boolean)
 * and confirmed (boolean | undefined), mapCommandToHttp('toggle_record', { subscriptionId, enabled, confirmed })
 * SHALL produce a POST request to /subscriptions/{id}/toggle-record with a JSON body
 * containing at minimum the enabled field, and SHALL include the confirmed field in the
 * body if and only if the confirmed argument is not undefined.
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import { mapCommandToHttp } from './transportRoutes';

describe('Feature: recording-ui-optimization, Property 2: toggle_record HTTP route mapping correctness', () => {
  it('produces correct POST method, path, and body for any valid inputs', () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 1, max: 1_000_000 }), // positive integer subscriptionId
        fc.boolean(), // enabled
        fc.option(fc.boolean()), // confirmed: boolean | undefined (fc.option wraps as T | null)
        (subscriptionId, enabled, confirmedOrNull) => {
          const confirmed = confirmedOrNull === null ? undefined : confirmedOrNull;

          const args: Record<string, unknown> = { subscriptionId, enabled };
          if (confirmed !== undefined) {
            args.confirmed = confirmed;
          }

          const result = mapCommandToHttp('toggle_record', args);

          // Method is always POST
          expect(result.method).toBe('POST');

          // Path matches /subscriptions/{id}/toggle-record
          expect(result.path).toBe(
            `/subscriptions/${encodeURIComponent(String(subscriptionId))}/toggle-record`,
          );

          // Body is valid JSON
          expect(result.body).toBeDefined();
          const body = JSON.parse(result.body!);

          // Body always contains enabled
          expect(body.enabled).toBe(enabled);

          // Body contains confirmed if and only if confirmed !== undefined
          if (confirmed !== undefined) {
            expect(body).toHaveProperty('confirmed');
            expect(body.confirmed).toBe(confirmed);
          } else {
            expect(body).not.toHaveProperty('confirmed');
          }
        },
      ),
      { numRuns: 100 },
    );
  }, 30_000);
});
