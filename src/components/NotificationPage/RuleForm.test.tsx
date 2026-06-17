import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// Mock the transport layer
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

import { RuleForm } from './RuleForm';
import { t } from '../../lib/i18n';
import type { EditRuleData } from '../../types';

const MOCK_SUBSCRIPTIONS = [
  { id: 1, sub_type: 'asset', symbol: 'BTC', display_name: 'Bitcoin', selected_provider_id: 'binance', asset_type: 'crypto', sort_order: 0, record_enabled: 1 },
  { id: 2, sub_type: 'asset', symbol: 'ETH', display_name: 'Ethereum', selected_provider_id: 'binance', asset_type: 'crypto', sort_order: 1, record_enabled: 1 },
  { id: 3, sub_type: 'asset', symbol: 'SOL', display_name: 'Solana', selected_provider_id: 'binance', asset_type: 'crypto', sort_order: 2, record_enabled: 1 },
];

const MOCK_CHANNELS = [
  { id: 1, channel_type: 'telegram', name: 'My Telegram', config: '{}', created_at: 1000 },
];

beforeEach(() => {
  mockInvoke.mockReset();
  mockInvoke.mockImplementation((cmd: string) => {
    switch (cmd) {
      case 'list_all_subscriptions':
        return Promise.resolve(MOCK_SUBSCRIPTIONS);
      case 'list_notification_channels':
        return Promise.resolve(MOCK_CHANNELS);
      case 'get_ai_provider_config':
        return Promise.resolve({ base_url: 'http://localhost', model: 'test', has_api_key: true });
      default:
        return Promise.resolve();
    }
  });
});

describe('RuleForm multi-select behavior', () => {
  const defaultProps = {
    onClose: vi.fn(),
    onSaved: vi.fn(),
  };

  describe('AI mode renders multi-select picker', () => {
    it('renders checkboxes for each subscription when AI mode is selected', async () => {
      render(<RuleForm {...defaultProps} />);

      // Switch to AI mode
      const aiModeBtn = screen.getByText(t.notifications.aiRule);
      fireEvent.click(aiModeBtn);

      // Wait for subscriptions to load and checkboxes to render
      await waitFor(() => {
        expect(screen.getByText(t.notifications.selectSubscriptions)).toBeInTheDocument();
      });

      // Verify checkboxes exist for each subscription
      const checkboxes = screen.getAllByRole('checkbox');
      // Filter to subscription checkboxes (exclude channel checkboxes)
      const subscriptionCheckboxes = checkboxes.filter(cb => {
        const label = cb.closest('label');
        return label?.classList.contains('subscription-checkbox');
      });
      expect(subscriptionCheckboxes).toHaveLength(MOCK_SUBSCRIPTIONS.length);
    });

    it('shows subscription symbols in checkbox labels', async () => {
      render(<RuleForm {...defaultProps} />);

      const aiModeBtn = screen.getByText(t.notifications.aiRule);
      fireEvent.click(aiModeBtn);

      await waitFor(() => {
        for (const sub of MOCK_SUBSCRIPTIONS) {
          expect(screen.getByText(`${sub.symbol} (${sub.selected_provider_id})`)).toBeInTheDocument();
        }
      });
    });
  });

  describe('Threshold mode renders single-select dropdown', () => {
    it('renders a <select> dropdown for subscription in threshold mode', async () => {
      const { container } = render(<RuleForm {...defaultProps} />);

      // Threshold mode is the default — wait for subscriptions to load
      await waitFor(() => {
        // Find the subscription label and its associated select
        const subscriptionLabel = screen.getByText(t.notifications.subscription);
        const formField = subscriptionLabel.closest('label');
        const selectEl = formField?.querySelector('select');
        expect(selectEl).not.toBeNull();
        expect(selectEl!.tagName).toBe('SELECT');
      });
    });

    it('does not render subscription checkboxes in threshold mode', async () => {
      render(<RuleForm {...defaultProps} />);

      await waitFor(() => {
        expect(screen.getByText(t.notifications.subscription)).toBeInTheDocument();
      });

      // Verify no subscription-checkbox elements exist
      const container = document.querySelector('.subscription-checkboxes');
      expect(container).toBeNull();
    });

    it('shows subscriptions as <option> elements in the dropdown', async () => {
      render(<RuleForm {...defaultProps} />);

      await waitFor(() => {
        for (const sub of MOCK_SUBSCRIPTIONS) {
          const option = screen.getByRole('option', { name: `${sub.symbol} (${sub.selected_provider_id})` });
          expect(option).toBeInTheDocument();
        }
      });
    });
  });

  describe('Validation error on zero subscriptions in AI mode', () => {
    it('shows validation error when submitting AI rule with no subscriptions selected', async () => {
      const user = userEvent.setup();
      render(<RuleForm {...defaultProps} />);

      // Switch to AI mode
      const aiModeBtn = screen.getByText(t.notifications.aiRule);
      await user.click(aiModeBtn);

      // Fill in required fields (name and prompt) but leave subscriptions empty
      await waitFor(() => {
        expect(screen.getByText(t.notifications.selectSubscriptions)).toBeInTheDocument();
      });

      const nameInput = screen.getByPlaceholderText(t.notifications.ruleNamePlaceholder);
      await user.type(nameInput, 'Test Rule');

      const promptTextarea = screen.getByPlaceholderText(t.notifications.promptPlaceholder);
      await user.type(promptTextarea, 'When price goes up');

      // Submit the form without selecting any subscriptions
      const submitBtn = screen.getByRole('button', { name: t.notifications.createRule });
      await user.click(submitBtn);

      // Verify validation error is displayed
      await waitFor(() => {
        expect(screen.getByText(t.notifications.subscriptionRequired)).toBeInTheDocument();
      });
    });

    it('does not call create_notification_rule when validation fails', async () => {
      const user = userEvent.setup();
      render(<RuleForm {...defaultProps} />);

      const aiModeBtn = screen.getByText(t.notifications.aiRule);
      await user.click(aiModeBtn);

      await waitFor(() => {
        expect(screen.getByText(t.notifications.selectSubscriptions)).toBeInTheDocument();
      });

      const nameInput = screen.getByPlaceholderText(t.notifications.ruleNamePlaceholder);
      await user.type(nameInput, 'Test Rule');

      const promptTextarea = screen.getByPlaceholderText(t.notifications.promptPlaceholder);
      await user.type(promptTextarea, 'When price goes up');

      const submitBtn = screen.getByRole('button', { name: t.notifications.createRule });
      await user.click(submitBtn);

      // Verify the transport was NOT called for rule creation
      expect(mockInvoke).not.toHaveBeenCalledWith('create_notification_rule', expect.anything());
    });
  });

  describe('Pre-selection when editing existing rule', () => {
    it('pre-selects subscriptions from editRule.subscription_ids in AI mode', async () => {
      const editRule: EditRuleData = {
        id: 42,
        name: 'My AI Rule',
        subscription_id: 1,
        subscription_ids: JSON.stringify([1, 3]),
        condition_type: 'ai',
        threshold: 0,
        channel_ids: '[]',
        cooldown_secs: 60,
        ai_config: JSON.stringify({ prompt: 'test prompt', history_window: 20, analysis_interval_secs: 300 }),
      };

      render(<RuleForm {...defaultProps} editRule={editRule} />);

      await waitFor(() => {
        expect(screen.getByText(t.notifications.selectSubscriptions)).toBeInTheDocument();
      });

      // Verify subscriptions 1 and 3 are checked, but subscription 2 is not
      const checkboxes = screen.getAllByRole('checkbox');
      const subscriptionCheckboxes = checkboxes.filter(cb => {
        const label = cb.closest('label');
        return label?.classList.contains('subscription-checkbox');
      });

      // Find checkbox for BTC (id=1) — should be checked
      const btcCheckbox = subscriptionCheckboxes.find(cb => {
        const label = cb.closest('label');
        return label?.textContent?.includes('BTC');
      }) as HTMLInputElement;
      expect(btcCheckbox.checked).toBe(true);

      // Find checkbox for ETH (id=2) — should NOT be checked
      const ethCheckbox = subscriptionCheckboxes.find(cb => {
        const label = cb.closest('label');
        return label?.textContent?.includes('ETH');
      }) as HTMLInputElement;
      expect(ethCheckbox.checked).toBe(false);

      // Find checkbox for SOL (id=3) — should be checked
      const solCheckbox = subscriptionCheckboxes.find(cb => {
        const label = cb.closest('label');
        return label?.textContent?.includes('SOL');
      }) as HTMLInputElement;
      expect(solCheckbox.checked).toBe(true);
    });

    it('falls back to editRule.subscription_id when subscription_ids is null', async () => {
      const editRule: EditRuleData = {
        id: 43,
        name: 'Legacy AI Rule',
        subscription_id: 2,
        subscription_ids: null,
        condition_type: 'ai',
        threshold: 0,
        channel_ids: '[]',
        cooldown_secs: 60,
        ai_config: JSON.stringify({ prompt: 'test prompt', history_window: 20, analysis_interval_secs: 300 }),
      };

      render(<RuleForm {...defaultProps} editRule={editRule} />);

      await waitFor(() => {
        expect(screen.getByText(t.notifications.selectSubscriptions)).toBeInTheDocument();
      });

      const checkboxes = screen.getAllByRole('checkbox');
      const subscriptionCheckboxes = checkboxes.filter(cb => {
        const label = cb.closest('label');
        return label?.classList.contains('subscription-checkbox');
      });

      // Only ETH (id=2) should be checked (fallback from subscription_id)
      const btcCheckbox = subscriptionCheckboxes.find(cb => {
        const label = cb.closest('label');
        return label?.textContent?.includes('BTC');
      }) as HTMLInputElement;
      expect(btcCheckbox.checked).toBe(false);

      const ethCheckbox = subscriptionCheckboxes.find(cb => {
        const label = cb.closest('label');
        return label?.textContent?.includes('ETH');
      }) as HTMLInputElement;
      expect(ethCheckbox.checked).toBe(true);

      const solCheckbox = subscriptionCheckboxes.find(cb => {
        const label = cb.closest('label');
        return label?.textContent?.includes('SOL');
      }) as HTMLInputElement;
      expect(solCheckbox.checked).toBe(false);
    });
  });
});
