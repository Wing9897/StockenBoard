/**
 * Property-based test for Transport Layer routing correctness (task 8.6).
 *
 * Feature: web-server-mode, Property 5: Transport layer routing correctness
 *
 * **Validates: Requirements 5.2, 5.3, 5.6**
 *
 * Property: For any backend command name and arguments, the Transport_Layer
 * SHALL route through Tauri IPC invoke when `window.__TAURI__` is present,
 * and through HTTP fetch to the corresponding REST endpoint when it is absent.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import * as fc from 'fast-check';
import { mapCommandToHttp, createTransport } from './transport';

// --- Mock @tauri-apps/api/core ---
const mockTauriInvoke = vi.fn().mockResolvedValue({ ok: true });
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockTauriInvoke(...args),
}));

// --- Known command names from the route map ---
const KNOWN_COMMANDS = [
  'list_all_subscriptions',
  'list_subscriptions',
  'add_subscription',
  'add_subscriptions_batch',
  'update_subscription',
  'remove_subscription',
  'remove_subscriptions',
  'list_views',
  'create_view',
  'rename_view',
  'delete_view',
  'add_sub_to_view',
  'remove_sub_from_view',
  'get_all_providers',
  'enable_provider',
  'list_provider_settings',
  'upsert_provider_settings',
  'has_api_key',
  'create_notification_rule',
  'list_notification_rules',
  'update_notification_rule',
  'delete_notification_rule',
  'toggle_notification_rule',
  'save_notification_channel',
  'list_notification_channels',
  'delete_notification_channel',
  'test_notification_channel',
  'get_notification_history',
  'get_notification_global_cooldown',
  'set_notification_global_cooldown',
  'save_ai_provider_config',
  'get_ai_provider_config',
  'test_ai_connection',
  'list_ai_models',
  'fetch_asset_price',
  'fetch_multiple_prices',
  'get_cached_prices',
  'get_poll_ticks',
  'get_price_history',
  'get_history_stats',
  'cleanup_history',
  'purge_all_history',
  'delete_subscription_history',
  'export_data',
  'import_data',
  'get_api_port',
  'set_api_port',
  'get_unattended_polling',
  'set_unattended_polling',
  'reload_polling',
  'reset_all_data',
  'get_data_dir',
  'set_icon',
  'remove_icon',
  'get_icons_dir',
  'download_logos',
  'lookup_dex_pool',
];

// --- Arbitraries ---

/** Generates a random known command name. */
const commandArb = fc.constantFrom(...KNOWN_COMMANDS);

/** Generates random args that cover fields used by various command mappers. */
const argsArb = fc.record({
  id: fc.string({ minLength: 1, maxLength: 20 }),
  type: fc.constantFrom('asset', 'dex', 'stock', 'crypto'),
  symbol: fc.string({ minLength: 1, maxLength: 10 }),
  provider: fc.string({ minLength: 1, maxLength: 15 }),
  address: fc.string({ minLength: 1, maxLength: 42 }),
  view_id: fc.string({ minLength: 1, maxLength: 20 }),
  sub_id: fc.string({ minLength: 1, maxLength: 20 }),
  name: fc.string({ minLength: 0, maxLength: 30 }),
});

describe('Property 5: Transport layer routing correctness', () => {
  let originalFetch: typeof globalThis.fetch;
  const mockFetch = vi.fn();

  beforeEach(() => {
    originalFetch = globalThis.fetch;
    mockFetch.mockReset();
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve({ data: { ok: true } }),
      text: () => Promise.resolve(''),
    });
    mockTauriInvoke.mockReset();
    mockTauriInvoke.mockResolvedValue({ ok: true });
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
    // Clean __TAURI_INTERNALS__ from window
    if ('__TAURI_INTERNALS__' in window) {
      delete (window as Record<string, unknown>).__TAURI_INTERNALS__;
    }
  });

  it('should route through Tauri IPC invoke when __TAURI_INTERNALS__ is present (100 runs)', async () => {
    // Simulate Tauri environment
    (window as Record<string, unknown>).__TAURI_INTERNALS__ = { version: '2.0' };

    const transport = createTransport();

    await fc.assert(
      fc.asyncProperty(commandArb, argsArb, async (command, args) => {
        mockTauriInvoke.mockResolvedValue({ result: 'ok' });

        const result = await transport.invoke(command, args);

        // The TauriTransport should have called Tauri's invoke with the
        // command name and args directly.
        expect(mockTauriInvoke).toHaveBeenLastCalledWith(command, args);

        // The result should come from Tauri invoke
        expect(result).toEqual({ result: 'ok' });
      }),
      { numRuns: 100 },
    );
  });

  it('should route through HTTP fetch when __TAURI_INTERNALS__ is absent (100 runs)', async () => {
    // Ensure NOT in Tauri environment
    if ('__TAURI_INTERNALS__' in window) {
      delete (window as Record<string, unknown>).__TAURI_INTERNALS__;
    }
    globalThis.fetch = mockFetch;

    const transport = createTransport();

    await fc.assert(
      fc.asyncProperty(commandArb, argsArb, async (command, args) => {
        mockFetch.mockResolvedValue({
          ok: true,
          status: 200,
          json: () => Promise.resolve({ data: { fetched: true } }),
          text: () => Promise.resolve(''),
        });

        const result = await transport.invoke(command, args);

        // Fetch should have been called exactly once for this invocation
        expect(mockFetch).toHaveBeenCalled();

        // Verify the fetch URL and method match mapCommandToHttp output
        const { method, path, body } = mapCommandToHttp(command, args);
        const lastCall = mockFetch.mock.calls[mockFetch.mock.calls.length - 1];
        const fetchUrl = lastCall[0] as string;
        const fetchOptions = lastCall[1] as RequestInit;

        // URL should be /api{path}
        expect(fetchUrl).toBe(`/api${path}`);
        // Method should match the mapped method
        expect(fetchOptions.method).toBe(method);
        // Body should match (if present in the mapping)
        if (body) {
          expect(fetchOptions.body).toBe(body);
        } else {
          expect(fetchOptions.body).toBeUndefined();
        }

        // The result should be unwrapped from the response envelope
        expect(result).toEqual({ fetched: true });
      }),
      { numRuns: 100 },
    );
  });

  it('should never call fetch when __TAURI_INTERNALS__ is present (100 runs)', async () => {
    // Simulate Tauri environment
    (window as Record<string, unknown>).__TAURI_INTERNALS__ = { version: '2.0' };
    globalThis.fetch = mockFetch;

    const transport = createTransport();

    await fc.assert(
      fc.asyncProperty(commandArb, argsArb, async (command, args) => {
        mockTauriInvoke.mockResolvedValue({ ok: true });
        mockFetch.mockClear();

        await transport.invoke(command, args);

        // Fetch must NOT be called when in Tauri mode
        expect(mockFetch).not.toHaveBeenCalled();
      }),
      { numRuns: 100 },
    );
  });

  it('should never call Tauri invoke when __TAURI_INTERNALS__ is absent (100 runs)', async () => {
    // Ensure NOT in Tauri environment
    if ('__TAURI_INTERNALS__' in window) {
      delete (window as Record<string, unknown>).__TAURI_INTERNALS__;
    }
    globalThis.fetch = mockFetch;

    const transport = createTransport();

    await fc.assert(
      fc.asyncProperty(commandArb, argsArb, async (command, args) => {
        mockTauriInvoke.mockClear();
        mockFetch.mockResolvedValue({
          ok: true,
          status: 200,
          json: () => Promise.resolve({ data: null }),
          text: () => Promise.resolve(''),
        });

        await transport.invoke(command, args);

        // Tauri invoke must NOT be called when NOT in Tauri mode
        expect(mockTauriInvoke).not.toHaveBeenCalled();
      }),
      { numRuns: 100 },
    );
  });
});
