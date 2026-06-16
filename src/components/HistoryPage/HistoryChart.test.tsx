/**
 * Unit tests for HistoryChart empty-data behavior and HistoryPage empty state rendering.
 *
 * Validates: Requirements 1.4, 1.7
 *
 * - When `records` is empty, the chart API (createChart) is NOT instantiated.
 * - When no data exists for a selected subscription, HistoryPage renders an empty
 *   state view with a placeholder icon and descriptive text.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';

// ── Hoisted mocks (available before vi.mock factories run) ──
const { mockCreateChart, mockRemove, mockInvoke } = vi.hoisted(() => {
  const mockRemove = vi.fn();
  const mockAddSeries = vi.fn(() => ({ setData: vi.fn() }));
  const mockTimeScale = vi.fn(() => ({ fitContent: vi.fn() }));
  const mockApplyOptions = vi.fn();
  const mockCreateChart = vi.fn(() => ({
    remove: mockRemove,
    addSeries: mockAddSeries,
    timeScale: mockTimeScale,
    applyOptions: mockApplyOptions,
  }));
  const mockInvoke = vi.fn(() => Promise.resolve(null));
  return { mockCreateChart, mockRemove, mockAddSeries, mockTimeScale, mockApplyOptions, mockInvoke };
});

// ── Mock lightweight-charts so createChart can be spied on ──
vi.mock('lightweight-charts', () => ({
  createChart: (...args: unknown[]) => mockCreateChart(...args),
  LineSeries: 'LineSeries',
}));

// ── Mock transport for HistoryPage ──
vi.mock('../../lib/transport', () => ({
  getTransport: () => ({
    invoke: (...args: unknown[]) => mockInvoke(...args),
    listen: () => () => {},
  }),
  createTransport: () => ({
    invoke: (...args: unknown[]) => mockInvoke(...args),
    listen: () => () => {},
  }),
  isTauri: () => false,
}));

// ── Mock ResizeObserver (not available in jsdom) ──
vi.stubGlobal('ResizeObserver', class {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
});

import { HistoryChart } from './HistoryChart';
import { HistoryPage } from './HistoryPage';
import { t } from '../../lib/i18n';
import type { Subscription } from '../../types';

describe('HistoryChart — empty data behavior', () => {
  beforeEach(() => {
    mockCreateChart.mockClear();
    mockRemove.mockClear();
  });

  it('does NOT call createChart when records array is empty', () => {
    render(<HistoryChart records={[]} session="regular" />);
    expect(mockCreateChart).not.toHaveBeenCalled();
  });

  it('does NOT call createChart when records have no entries (pre session)', () => {
    render(<HistoryChart records={[]} session="pre" />);
    expect(mockCreateChart).not.toHaveBeenCalled();
  });

  it('does NOT call createChart when records have no entries (post session)', () => {
    render(<HistoryChart records={[]} session="post" />);
    expect(mockCreateChart).not.toHaveBeenCalled();
  });

  it('DOES call createChart when records contain data', () => {
    const records = [
      { id: 1, subscription_id: 1, provider_id: 'binance', price: 100, change_pct: null, volume: null, pre_price: null, post_price: null, recorded_at: 1700000000 },
    ];
    render(<HistoryChart records={records} session="regular" />);
    expect(mockCreateChart).toHaveBeenCalledTimes(1);
  });
});

describe('HistoryPage — empty state view when no data exists', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  /** Helper: set up transport mock to return subscriptions and empty history */
  function setupEmptyHistory(assetSubs: Subscription[]) {
    mockInvoke.mockImplementation((cmd: string, args?: { subType?: string }) => {
      switch (cmd) {
        case 'list_subscriptions':
          // HistoryPage calls this twice: once for 'asset', once for 'dex'
          if (args?.subType === 'asset') return Promise.resolve(assetSubs);
          return Promise.resolve([]); // no dex subs
        case 'get_price_history':
          return Promise.resolve([]); // No records
        default:
          return Promise.resolve(null);
      }
    });
  }

  const mockToast = {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  };

  const testSub: Subscription = {
    id: 1,
    sub_type: 'asset',
    symbol: 'BTC',
    display_name: 'Bitcoin',
    selected_provider_id: 'binance',
    asset_type: 'crypto',
    sort_order: 0,
    record_enabled: 0,
    record_from_hour: null,
    record_to_hour: null,
  };

  it('renders full-page empty state with icon and text when no subscriptions exist', async () => {
    mockInvoke.mockImplementation(() => Promise.resolve([]));

    const { container } = render(<HistoryPage onToast={mockToast} />);

    await waitFor(() => {
      const emptyFull = container.querySelector('.history-full-empty');
      expect(emptyFull).not.toBeNull();
    });

    // Verify placeholder icon is rendered
    const icon = container.querySelector('.history-empty-icon');
    expect(icon).not.toBeNull();
    expect(icon!.textContent).toBeTruthy();

    // Verify descriptive text is present
    expect(screen.getByText(t.history.noSubs)).toBeInTheDocument();
  });

  it('renders empty state with placeholder icon and descriptive text when selected subscription has no data', async () => {
    setupEmptyHistory([testSub]);

    const { container } = render(<HistoryPage onToast={mockToast} />);

    // Wait for subs to load
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('list_subscriptions', expect.anything());
    });

    // Select the subscription by clicking on the sub item
    const subItem = await screen.findByText('Bitcoin');
    subItem.closest('.history-sub-item')?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    subItem.click();

    // Wait for empty state to appear (no data for the selected subscription)
    await waitFor(() => {
      const emptyState = container.querySelector('.history-empty-state');
      expect(emptyState).not.toBeNull();
    });

    // Verify placeholder icon
    const emptyState = container.querySelector('.history-empty-state');
    const icon = emptyState!.querySelector('.history-empty-icon');
    expect(icon).not.toBeNull();
    expect(icon!.textContent).toBeTruthy();

    // Verify descriptive text indicating no data
    expect(screen.getByText(t.history.noData)).toBeInTheDocument();
  });
});
