import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';

// Mock the transport layer so child components can render without a backend.
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

import { t } from '../../lib/i18n';
import { NotificationPage } from './NotificationPage';

beforeEach(() => {
  mockInvoke.mockReset();
  // Default backend responses so child components don't throw.
  mockInvoke.mockImplementation((cmd: string) => {
    switch (cmd) {
      case 'list_notification_rules':
        return Promise.resolve([]);
      case 'get_notification_global_cooldown':
        return Promise.resolve(30);
      default:
        return Promise.resolve();
    }
  });
});

describe('NotificationPage tab structure', () => {
  it('renders exactly 3 tabs in correct order: rules, channels, ai-settings', () => {
    render(<NotificationPage />);

    const tabs = screen.getAllByRole('button', { name: /./i }).filter(btn =>
      btn.classList.contains('notification-tab')
    );

    expect(tabs).toHaveLength(3);
    expect(tabs[0]).toHaveTextContent(t.notifications.rules);
    expect(tabs[1]).toHaveTextContent(t.notifications.channels);
    expect(tabs[2]).toHaveTextContent(t.notifications.aiSettings);
  });

  it('does not render NotificationHistory for any tab', () => {
    const { container } = render(<NotificationPage />);

    // NotificationHistory had a distinctive class; verify it is absent.
    expect(container.querySelector('.notification-history')).toBeNull();

    // Also verify no "history" content panel is rendered
    const tabs = container.querySelectorAll('.notification-tab');
    tabs.forEach(tab => {
      // Click each tab to verify no history content appears
      tab.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });
    expect(container.querySelector('.notification-history')).toBeNull();
  });

  it('default active tab is rules', () => {
    render(<NotificationPage />);

    const tabs = screen.getAllByRole('button').filter(btn =>
      btn.classList.contains('notification-tab')
    );

    // The first tab (rules) should have the 'active' class
    const rulesTab = tabs.find(tab => tab.textContent === t.notifications.rules);
    expect(rulesTab).toBeDefined();
    expect(rulesTab!.classList.contains('active')).toBe(true);

    // Other tabs should not be active
    const channelsTab = tabs.find(tab => tab.textContent === t.notifications.channels);
    const aiTab = tabs.find(tab => tab.textContent === t.notifications.aiSettings);
    expect(channelsTab!.classList.contains('active')).toBe(false);
    expect(aiTab!.classList.contains('active')).toBe(false);
  });
});
