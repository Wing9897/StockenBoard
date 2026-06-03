import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';

// Mock the Tauri IPC + event layers before importing the hook.
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({
  // listen resolves to an unlisten fn; we record calls for assertions.
  listen: vi.fn(async () => () => {}),
}));

import { invoke } from '@tauri-apps/api/core';
import { useAssetData } from './useAssetData';

const mockInvoke = invoke as unknown as Mock;

const WS_SUB = {
  id: 1,
  sub_type: 'asset',
  symbol: 'BTCUSDT',
  selected_provider_id: 'binance',
  asset_type: 'crypto',
  sort_order: 0,
  record_enabled: 0,
};

/** Route invoke by command name to the values the init sequence expects. */
function setupInvoke() {
  mockInvoke.mockImplementation((cmd: string) => {
    switch (cmd) {
      case 'get_all_providers':
        return Promise.resolve([]);
      case 'list_subscriptions':
        return Promise.resolve([WS_SUB]);
      case 'get_cached_prices':
        return Promise.resolve([]);
      case 'get_poll_ticks':
        return Promise.resolve([]);
      case 'list_provider_settings':
        // binance flagged as websocket so the hook starts a WS stream.
        return Promise.resolve([{ provider_id: 'binance', connection_type: 'websocket' }]);
      default:
        return Promise.resolve(undefined);
    }
  });
}

beforeEach(() => {
  mockInvoke.mockReset();
  setupInvoke();
});

describe('useAssetData WebSocket lifecycle', () => {
  it('starts a WS stream for websocket-enabled providers on mount', async () => {
    const { result } = renderHook(() => useAssetData('asset'));
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(mockInvoke).toHaveBeenCalledWith('start_ws_stream', {
      providerId: 'binance',
      symbols: ['BTCUSDT'],
    });
  });

  it('stops the WS stream it started when the hook unmounts (no leak)', async () => {
    const { result, unmount } = renderHook(() => useAssetData('asset'));
    await waitFor(() => expect(result.current.loading).toBe(false));
    // Ensure the stream actually started before we assert teardown.
    expect(mockInvoke).toHaveBeenCalledWith('start_ws_stream', expect.anything());

    unmount();

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('stop_ws_stream', { providerId: 'binance' });
    });
  });

  it('does not start any WS stream for the dex scope', async () => {
    const { result } = renderHook(() => useAssetData('dex'));
    await waitFor(() => expect(result.current.loading).toBe(false));

    const startCalls = mockInvoke.mock.calls.filter(c => c[0] === 'start_ws_stream');
    expect(startCalls).toHaveLength(0);
  });
});
