import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { RuleForm } from './RuleForm';
import { t } from '../../lib/i18n';
import type { Subscription, EditRuleData } from '../../types';

// RuleForm loads subscriptions through loadAllSubscriptions(), which wraps
// getTransport().invoke('list_all_subscriptions'). It also calls getTransport().invoke('list_notification_channels')
// and getTransport().invoke('get_ai_provider_config') on mount. We mock the transport module so
// invoke resolves/rejects per command name without touching a real backend.
const mockInvoke = vi.fn();
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

/** Build a complete Subscription with sane defaults so tests stay focused. */
function makeSubscription(overrides: Partial<Subscription> & Pick<Subscription, 'id' | 'symbol' | 'selected_provider_id'>): Subscription {
  return {
    sub_type: 'asset',
    asset_type: 'crypto',
    sort_order: 0,
    record_enabled: 0,
    ...overrides,
  } as Subscription;
}

/** asset + dex mix, matching the notification engine's list_all_subscriptions behavior. */
const SUBSCRIPTIONS: Subscription[] = [
  makeSubscription({ id: 1, sub_type: 'asset', symbol: 'BTC', selected_provider_id: 'binance' }),
  makeSubscription({ id: 2, sub_type: 'asset', symbol: 'ETH', selected_provider_id: 'coinbase' }),
  makeSubscription({ id: 3, sub_type: 'dex', symbol: 'WETH/USDC', selected_provider_id: 'uniswap' }),
];

/** Locate the subscription <select> via its placeholder option (form-fields have no htmlFor). */
function getSubscriptionSelect(): HTMLSelectElement {
  const placeholder = screen.getByRole('option', { name: t.notifications.selectSubscription });
  const select = placeholder.closest('select');
  if (!select) throw new Error('subscription select not found');
  return select as HTMLSelectElement;
}

/** Default happy-path invoke behaviour; individual tests can override. */
function setInvoke(subsResult: Promise<Subscription[]>) {
  mockInvoke.mockImplementation((cmd: string) => {
    switch (cmd) {
      case 'list_all_subscriptions':
        return subsResult;
      case 'list_notification_channels':
        return Promise.resolve([]);
      case 'get_ai_provider_config':
        return Promise.resolve(null);
      default:
        return Promise.resolve(null);
    }
  });
}

describe('RuleForm subscription dropdown', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it('renders asset + dex subscriptions as options with symbol/provider labels (1.1, 1.2, 1.3, 1.4)', async () => {
    setInvoke(Promise.resolve(SUBSCRIPTIONS));

    render(<RuleForm onClose={() => {}} onSaved={() => {}} />);

    // Wait until the async subscription load has populated the options.
    const btcOption = await screen.findByRole('option', { name: 'BTC (binance)' });
    expect(btcOption).toBeInTheDocument();

    // asset + dex both appear with the "{symbol} ({provider})" label.
    expect(screen.getByRole('option', { name: 'ETH (coinbase)' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'WETH/USDC (uniswap)' })).toBeInTheDocument();

    // The subscription <select> holds the placeholder + one option per subscription.
    const select = getSubscriptionSelect();
    expect(select.options).toHaveLength(SUBSCRIPTIONS.length + 1);
    expect(select.options[0].textContent).toBe(t.notifications.selectSubscription);
  });

  it('preselects the editRule subscription_id in the <select> (1.6)', async () => {
    setInvoke(Promise.resolve(SUBSCRIPTIONS));

    const editRule: EditRuleData = {
      id: 42,
      name: 'BTC alert',
      subscription_id: 3,
      condition_type: 'price_above',
      threshold: 65000,
      channel_ids: '[]',
      cooldown_secs: 300,
      ai_config: null,
    };

    render(<RuleForm onClose={() => {}} onSaved={() => {}} editRule={editRule} />);

    // Wait for options to render so the select can resolve its value.
    await screen.findByRole('option', { name: 'WETH/USDC (uniswap)' });

    const select = getSubscriptionSelect();
    await waitFor(() => expect(select.value).toBe('3'));
  });

  it('shows a visible error message when loading subscriptions fails (1.5)', async () => {
    const message = '無法載入訂閱列表';
    setInvoke(Promise.reject(message));

    const { container } = render(<RuleForm onClose={() => {}} onSaved={() => {}} />);

    // The error must surface in the visible .rule-form-error region, not just console.
    await waitFor(() => {
      const errorEl = container.querySelector('.rule-form-error');
      expect(errorEl).not.toBeNull();
      expect(errorEl?.textContent).toContain(message);
    });
  });
});
