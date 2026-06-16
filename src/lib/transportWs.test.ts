import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import fc from 'fast-check';

// Feature: web-mode-file-operations, Property 6: Web mode command interception

vi.mock('./webFileOps', () => ({
  webModeHandlers: {
    set_icon: vi.fn().mockResolvedValue({ path: '/icons/test.png' }),
    export_file: vi.fn().mockResolvedValue(undefined),
    import_file: vi.fn().mockResolvedValue('{"data":"test"}'),
  },
}));

describe('HttpTransport - Web mode command interception', () => {
  let fetchSpy: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.resetAllMocks();

    // Mock global fetch for non-intercepted commands
    fetchSpy = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve({ data: null }),
      text: () => Promise.resolve(''),
    });
    vi.stubGlobal('fetch', fetchSpy);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  // **Validates: Requirements 5.1, 5.2**
  it('Property 6: Intercepted commands call web handler and NOT fetch', async () => {
    const { HttpTransport } = await import('./transportWs');
    const { webModeHandlers } = await import('./webFileOps');

    const interceptedCommands = ['set_icon', 'export_file', 'import_file'] as const;

    await fc.assert(
      fc.asyncProperty(
        fc.constantFrom(...interceptedCommands),
        fc.dictionary(
          fc.string({ minLength: 1, maxLength: 20 }).filter(s => /^[a-zA-Z_]+$/.test(s)),
          fc.oneof(fc.string(), fc.integer(), fc.boolean())
        ),
        async (command, args) => {
          // Reset mocks for each iteration
          fetchSpy.mockClear();
          const handler = webModeHandlers[command] as ReturnType<typeof vi.fn>;
          handler.mockClear();

          // Re-setup mock return values after clear
          if (command === 'set_icon') {
            handler.mockResolvedValue({ path: '/icons/test.png' });
          } else if (command === 'export_file') {
            handler.mockResolvedValue(undefined);
          } else {
            handler.mockResolvedValue('{"data":"test"}');
          }

          const transport = new HttpTransport();
          await transport.invoke(command, args);

          // Web handler SHOULD have been called
          expect(handler).toHaveBeenCalledWith(args);

          // fetch SHOULD NOT have been called
          expect(fetchSpy).not.toHaveBeenCalled();
        }
      ),
      { numRuns: 100 }
    );
  });

  it('Property 6: Non-intercepted commands call fetch and NOT web handlers', async () => {
    const { HttpTransport } = await import('./transportWs');
    const { webModeHandlers } = await import('./webFileOps');

    // Commands that are NOT in the intercepted set
    const nonInterceptedCommands = [
      'list_all_subscriptions',
      'export_data',
      'get_all_providers',
      'get_cached_prices',
      'list_views',
    ] as const;

    await fc.assert(
      fc.asyncProperty(
        fc.constantFrom(...nonInterceptedCommands),
        async (command) => {
          // Reset mocks for each iteration
          fetchSpy.mockClear();
          for (const key of Object.keys(webModeHandlers)) {
            (webModeHandlers[key] as ReturnType<typeof vi.fn>).mockClear();
          }

          const transport = new HttpTransport();
          await transport.invoke(command);

          // fetch SHOULD have been called
          expect(fetchSpy).toHaveBeenCalled();

          // No web handler should have been called
          for (const key of Object.keys(webModeHandlers)) {
            expect(webModeHandlers[key]).not.toHaveBeenCalled();
          }
        }
      ),
      { numRuns: 100 }
    );
  });
});
