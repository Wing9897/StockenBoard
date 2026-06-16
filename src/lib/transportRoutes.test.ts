/**
 * Bug Condition Exploration Test — Route Mapper Parameter Name Mismatch
 *
 * **Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 1.10, 1.11, 1.12, 1.13, 1.14**
 *
 * This test encodes the EXPECTED (correct) behavior for the 14 affected route mappers.
 * On UNFIXED code, these tests MUST FAIL — failure confirms the bug exists.
 * After the fix is applied, these tests will PASS — confirming the fix works.
 */
import { describe, it, expect } from 'vitest';
import fc from 'fast-check';
import { mapCommandToHttp } from './transportRoutes';

describe('Bug Condition Exploration: Route Mapper Parameter Name Mismatch', () => {
  /**
   * Property 1: Bug Condition — Route Mapper Produces Correct HTTP Tuple
   *
   * For any input where the command is one of the 14 affected commands and args
   * contain valid camelCase parameters, mapCommandToHttp SHALL produce correct
   * method, path, and body.
   */

  it('list_subscriptions: reads subType for query param (not a.type)', () => {
    fc.assert(
      fc.property(
        fc.constantFrom('asset', 'dex', 'crypto', 'stock'),
        (subType) => {
          const result = mapCommandToHttp('list_subscriptions', { subType });
          expect(result.path).toContain(`?type=${subType}`);
          expect(result.method).toBe('GET');
        }
      )
    );
  });

  it('list_views: reads viewType for query param (not a.type)', () => {
    fc.assert(
      fc.property(
        fc.constantFrom('asset', 'dex'),
        (viewType) => {
          const result = mapCommandToHttp('list_views', { viewType });
          expect(result.path).toContain(`?type=${viewType}`);
          expect(result.method).toBe('GET');
        }
      )
    );
  });

  it('has_api_key: reads providerId for path (not a.id)', () => {
    fc.assert(
      fc.property(
        fc.constantFrom('binance', 'coingecko', 'coinmarketcap', 'uniswap'),
        (providerId) => {
          const result = mapCommandToHttp('has_api_key', { providerId });
          expect(result.path).toBe(`/provider-settings/${providerId}/has-key`);
          expect(result.path).not.toContain('undefined');
          expect(result.method).toBe('GET');
        }
      )
    );
  });

  it('toggle_record: reads subscriptionId for path (not a.subscription_id)', () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 1, max: 1000 }),
        fc.boolean(),
        (subscriptionId, enabled) => {
          const result = mapCommandToHttp('toggle_record', { subscriptionId, enabled });
          expect(result.path).toContain(`/subscriptions/${subscriptionId}/toggle-record`);
          expect(result.path).not.toContain('undefined');
          expect(result.method).toBe('POST');
        }
      )
    );
  });

  it('set_record_hours: reads subscriptionId for path and body uses snake_case keys', () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 1, max: 1000 }),
        fc.integer({ min: 0, max: 23 }),
        fc.integer({ min: 0, max: 23 }),
        (subscriptionId, fromHour, toHour) => {
          const result = mapCommandToHttp('set_record_hours', { subscriptionId, fromHour, toHour });
          expect(result.path).toContain(`/subscriptions/${subscriptionId}/`);
          expect(result.path).not.toContain('undefined');
          const body = JSON.parse(result.body!);
          expect(body).toHaveProperty('from_hour', fromHour);
          expect(body).toHaveProperty('to_hour', toHour);
        }
      )
    );
  });

  it('upsert_provider_settings: reads providerId for path and body has snake_case keys', () => {
    fc.assert(
      fc.property(
        fc.constantFrom('binance', 'coingecko', 'coinmarketcap'),
        fc.string({ minLength: 5, maxLength: 20 }),
        fc.integer({ min: 1, max: 3600 }),
        (providerId, apiKey, refreshInterval) => {
          const result = mapCommandToHttp('upsert_provider_settings', {
            providerId,
            apiKey,
            refreshInterval,
          });
          expect(result.path).toContain(`/provider-settings/${providerId}`);
          expect(result.path).not.toContain('undefined');
          expect(result.method).toBe('PUT');
          const body = JSON.parse(result.body!);
          expect(body).toHaveProperty('api_key', apiKey);
          expect(body).toHaveProperty('refresh_interval', refreshInterval);
        }
      )
    );
  });

  it('add_sub_to_view: reads viewId for path and body contains only subscription_id', () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 1, max: 1000 }),
        fc.integer({ min: 1, max: 1000 }),
        (viewId, subscriptionId) => {
          const result = mapCommandToHttp('add_sub_to_view', { viewId, subscriptionId });
          expect(result.path).toBe(`/views/${viewId}/subscriptions`);
          expect(result.path).not.toContain('undefined');
          expect(result.method).toBe('POST');
          const body = JSON.parse(result.body!);
          expect(body).toEqual({ subscription_id: subscriptionId });
        }
      )
    );
  });

  it('remove_sub_from_view: reads viewId and subscriptionId for path (not a.view_id, a.sub_id)', () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 1, max: 1000 }),
        fc.integer({ min: 1, max: 1000 }),
        (viewId, subscriptionId) => {
          const result = mapCommandToHttp('remove_sub_from_view', { viewId, subscriptionId });
          expect(result.path).toBe(`/views/${viewId}/subscriptions/${subscriptionId}`);
          expect(result.path).not.toContain('undefined');
          expect(result.method).toBe('DELETE');
        }
      )
    );
  });

  it('add_subscription: body uses snake_case keys', () => {
    fc.assert(
      fc.property(
        fc.constantFrom('asset', 'dex'),
        fc.string({ minLength: 3, maxLength: 10 }),
        fc.string({ minLength: 3, maxLength: 20 }),
        fc.constantFrom('binance', 'coingecko'),
        fc.constantFrom('crypto', 'stock'),
        (subType, symbol, displayName, providerId, assetType) => {
          const result = mapCommandToHttp('add_subscription', {
            subType,
            symbol,
            displayName,
            providerId,
            assetType,
          });
          expect(result.method).toBe('POST');
          const body = JSON.parse(result.body!);
          expect(body).toHaveProperty('sub_type', subType);
          expect(body).toHaveProperty('display_name', displayName);
          expect(body).toHaveProperty('provider_id', providerId);
          expect(body).toHaveProperty('asset_type', assetType);
          // Should NOT have camelCase keys
          expect(body).not.toHaveProperty('subType');
          expect(body).not.toHaveProperty('displayName');
          expect(body).not.toHaveProperty('providerId');
          expect(body).not.toHaveProperty('assetType');
        }
      )
    );
  });

  it('update_subscription: body uses snake_case keys', () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 1, max: 1000 }),
        fc.string({ minLength: 3, maxLength: 10 }),
        fc.string({ minLength: 3, maxLength: 20 }),
        fc.constantFrom('binance', 'coingecko'),
        fc.constantFrom('crypto', 'stock'),
        (id, symbol, displayName, providerId, assetType) => {
          const result = mapCommandToHttp('update_subscription', {
            id,
            symbol,
            displayName,
            providerId,
            assetType,
          });
          expect(result.method).toBe('PUT');
          expect(result.path).toContain(`/subscriptions/${id}`);
          const body = JSON.parse(result.body!);
          expect(body).toHaveProperty('display_name', displayName);
          expect(body).toHaveProperty('provider_id', providerId);
          expect(body).toHaveProperty('asset_type', assetType);
          // Should NOT have camelCase keys
          expect(body).not.toHaveProperty('displayName');
          expect(body).not.toHaveProperty('providerId');
          expect(body).not.toHaveProperty('assetType');
        }
      )
    );
  });

  it('create_view: body contains { name, type } (not camelCase viewType)', () => {
    fc.assert(
      fc.property(
        fc.string({ minLength: 1, maxLength: 30 }),
        fc.constantFrom('asset', 'dex'),
        (name, viewType) => {
          const result = mapCommandToHttp('create_view', { name, viewType });
          expect(result.method).toBe('POST');
          const body = JSON.parse(result.body!);
          expect(body).toEqual({ name, type: viewType });
        }
      )
    );
  });

  it('add_subscriptions_batch: body is unwrapped array with snake_case keys', () => {
    fc.assert(
      fc.property(
        fc.array(
          fc.record({
            subType: fc.constantFrom('asset', 'dex'),
            symbol: fc.string({ minLength: 3, maxLength: 10 }),
          }),
          { minLength: 1, maxLength: 5 }
        ),
        (items) => {
          const result = mapCommandToHttp('add_subscriptions_batch', { items });
          expect(result.method).toBe('POST');
          const body = JSON.parse(result.body!);
          // Body should be an array, not wrapped in { items: [...] }
          expect(Array.isArray(body)).toBe(true);
          // Each item should have snake_case keys
          for (const item of body) {
            expect(item).toHaveProperty('sub_type');
            expect(item).not.toHaveProperty('subType');
          }
        }
      )
    );
  });

  it('set_notification_global_cooldown: body format matches backend expectation', () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 0, max: 86400 }),
        (secs) => {
          const result = mapCommandToHttp('set_notification_global_cooldown', { secs });
          expect(result.method).toBe('PUT');
          expect(result.path).toBe('/notifications/cooldown');
          const body = JSON.parse(result.body!);
          expect(body).toHaveProperty('seconds', secs);
        }
      )
    );
  });
});

/**
 * Preservation Property Tests — Unaffected Routes Produce Identical Output
 *
 * **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9**
 *
 * These tests verify that routes NOT affected by the bug continue to produce
 * correct, unchanged output after any fix is applied. They run on the UNFIXED
 * code first (must PASS) to establish a baseline, then are re-run after the fix
 * to confirm no regressions.
 */
describe('Preservation: Unaffected Routes Produce Identical Output', () => {
  it('list_all_subscriptions: always returns GET /subscriptions regardless of args', () => {
    fc.assert(
      fc.property(fc.constant(undefined), () => {
        const result = mapCommandToHttp('list_all_subscriptions', {});
        expect(result).toEqual({ method: 'GET', path: '/subscriptions' });
      })
    );
  });

  it('remove_subscription: DELETE /subscriptions/:id for any positive integer id', () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 1, max: 100000 }),
        (id) => {
          const result = mapCommandToHttp('remove_subscription', { id });
          expect(result).toEqual({
            method: 'DELETE',
            path: `/subscriptions/${encodeURIComponent(String(id))}`,
          });
        }
      )
    );
  });

  it('remove_subscriptions: DELETE /subscriptions/batch with ids array in body', () => {
    fc.assert(
      fc.property(
        fc.array(fc.integer({ min: 1, max: 100000 }), { minLength: 1, maxLength: 10 }),
        (ids) => {
          const args = { ids };
          const result = mapCommandToHttp('remove_subscriptions', args);
          expect(result).toEqual({
            method: 'DELETE',
            path: '/subscriptions/batch',
            body: JSON.stringify(args),
          });
        }
      )
    );
  });

  it('get_all_providers: always returns GET /providers regardless of args', () => {
    fc.assert(
      fc.property(fc.constant(undefined), () => {
        const result = mapCommandToHttp('get_all_providers', {});
        expect(result).toEqual({ method: 'GET', path: '/providers' });
      })
    );
  });

  it('get_cached_prices: always returns GET /prices/cached regardless of args', () => {
    fc.assert(
      fc.property(fc.constant(undefined), () => {
        const result = mapCommandToHttp('get_cached_prices', {});
        expect(result).toEqual({ method: 'GET', path: '/prices/cached' });
      })
    );
  });

  it('rename_view: PUT /views/:id with full args as body for any id and name', () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 1, max: 100000 }),
        fc.string({ minLength: 1, maxLength: 50 }),
        (id, name) => {
          const args = { id, name };
          const result = mapCommandToHttp('rename_view', args);
          expect(result).toEqual({
            method: 'PUT',
            path: `/views/${encodeURIComponent(String(id))}`,
            body: JSON.stringify(args),
          });
        }
      )
    );
  });

  it('delete_view: DELETE /views/:id for any positive integer id', () => {
    fc.assert(
      fc.property(
        fc.integer({ min: 1, max: 100000 }),
        (id) => {
          const result = mapCommandToHttp('delete_view', { id });
          expect(result).toEqual({
            method: 'DELETE',
            path: `/views/${encodeURIComponent(String(id))}`,
          });
        }
      )
    );
  });

  it('fetch_asset_price: GET /prices/fetch/:provider/:symbol for any providerId and symbol', () => {
    fc.assert(
      fc.property(
        fc.constantFrom('binance', 'coingecko', 'coinmarketcap', 'uniswap'),
        fc.string({ minLength: 1, maxLength: 20 }).filter((s) => s.trim().length > 0),
        (providerId, symbol) => {
          const result = mapCommandToHttp('fetch_asset_price', { providerId, symbol });
          expect(result).toEqual({
            method: 'GET',
            path: `/prices/fetch/${encodeURIComponent(String(providerId))}/${encodeURIComponent(String(symbol))}`,
          });
        }
      )
    );
  });
});
