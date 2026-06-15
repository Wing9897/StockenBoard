import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import { render, screen, waitFor, fireEvent, cleanup } from '@testing-library/react';
import type { NotificationRuleRow } from '../../types';

// Mock the transport layer so the component can run in jsdom without a backend.
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

import { t, setLocale } from '../../lib/i18n';
import { RuleList } from './RuleList';

function makeRule(overrides: Partial<NotificationRuleRow> = {}): NotificationRuleRow {
  return {
    id: 1,
    name: 'Rule A',
    subscription_id: 10,
    condition_type: 'price_above',
    threshold: 100,
    channel_ids: '[]',
    cooldown_secs: 0,
    enabled: true,
    ai_config: null,
    created_at: 0,
    updated_at: 0,
    ...overrides,
  };
}

/** Configure invoke: list returns the given rules, delete/toggle resolve. */
function setupInvoke(rules: NotificationRuleRow[]) {
  mockInvoke.mockImplementation((cmd: string) => {
    switch (cmd) {
      case 'list_notification_rules':
        return Promise.resolve(rules);
      case 'delete_notification_rule':
        return Promise.resolve();
      case 'toggle_notification_rule':
        return Promise.resolve();
      default:
        return Promise.resolve();
    }
  });
}

beforeEach(async () => {
  mockInvoke.mockReset();
  // Keep tests deterministic: always start from the base locale.
  await setLocale('zh_TW');
});

describe('RuleList delete confirmation flow', () => {
  it('shows ConfirmDialog when delete is clicked and calls delete invoke on confirm', async () => {
    setupInvoke([makeRule({ id: 7 })]);
    render(<RuleList />);

    // Wait for the async rule load to finish rendering.
    await screen.findByText('Rule A');

    // Click the delete (🗑) button — identified by its localized title.
    fireEvent.click(screen.getByTitle(t.common.delete));

    // The unified ConfirmDialog should appear with the i18n confirm message.
    expect(await screen.findByText(t.notifications.deleteConfirm)).toBeInTheDocument();

    // Confirm the deletion.
    fireEvent.click(screen.getByRole('button', { name: t.common.confirm }));

    // The backend delete command should be invoked with the rule id.
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('delete_notification_rule', { id: 7 });
    });
  });

  it('does not call delete invoke when the confirmation is cancelled', async () => {
    setupInvoke([makeRule({ id: 7 })]);
    render(<RuleList />);

    await screen.findByText('Rule A');

    fireEvent.click(screen.getByTitle(t.common.delete));
    expect(await screen.findByText(t.notifications.deleteConfirm)).toBeInTheDocument();

    // Cancel instead of confirming.
    fireEvent.click(screen.getByRole('button', { name: t.common.cancel }));

    // Dialog should close and no delete invoke should ever be issued.
    await waitFor(() => {
      expect(screen.queryByText(t.notifications.deleteConfirm)).not.toBeInTheDocument();
    });
    expect(mockInvoke).not.toHaveBeenCalledWith(
      'delete_notification_rule',
      expect.anything(),
    );
  });
});

describe('RuleList i18n condition summary', () => {
  it('renders the condition summary in the active locale and updates when locale changes', async () => {
    setupInvoke([makeRule({ condition_type: 'price_above', threshold: 100 })]);
    const { rerender } = render(<RuleList />);

    await screen.findByText('Rule A');

    // Base locale (zh_TW): condPriceAbove(v) => `價格 > $${v}`
    expect(screen.getByText('價格 > $100')).toBeInTheDocument();
    expect(screen.queryByText('Price > $100')).not.toBeInTheDocument();

    // Switch language and re-render — the summary text should follow the locale.
    await setLocale('en');
    rerender(<RuleList />);

    await waitFor(() => {
      expect(screen.getByText('Price > $100')).toBeInTheDocument();
    });
    expect(screen.queryByText('價格 > $100')).not.toBeInTheDocument();

    // Restore base locale so other tests/suites are unaffected.
    await setLocale('zh_TW');
    cleanup();
  });
});
