import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor, act, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// Mock the transport layer. The component imports `getTransport` from
// '../../lib/transport'; route each command to a controllable mock so the
// tests never touch a real backend.
const invokeMock = vi.fn();
vi.mock('../../lib/transport', () => ({
  getTransport: () => ({
    invoke: (...args: unknown[]) => invokeMock(...args),
    listen: () => () => {},
  }),
  createTransport: () => ({
    invoke: (...args: unknown[]) => invokeMock(...args),
    listen: () => () => {},
  }),
  isTauri: () => false,
}));

import { ChannelSettings } from './ChannelSettings';
import { t, setLocale } from '../../lib/i18n';

// A representative channel list returned by `list_notification_channels`.
const CHANNELS = [
  { id: 1, channel_type: 'telegram', name: 'My Telegram', config: '{}', created_at: 0 },
  { id: 2, channel_type: 'webhook', name: 'My Webhook', config: '{}', created_at: 0 },
];

/** Default invoke behaviour: serve the channel list, resolve everything else. */
function defaultInvoke(cmd: string): Promise<unknown> {
  if (cmd === 'list_notification_channels') return Promise.resolve(CHANNELS);
  return Promise.resolve();
}

beforeEach(async () => {
  invokeMock.mockReset();
  invokeMock.mockImplementation((cmd: string) => defaultInvoke(cmd));
  // Keep tests isolated from any locale persisted by a previous test.
  await act(async () => { await setLocale('zh_TW'); });
});

afterEach(() => {
  cleanup();
});

describe('ChannelSettings — delete confirmation flow', () => {
  it('shows the ConfirmDialog when a channel delete button is clicked', async () => {
    const user = userEvent.setup();
    render(<ChannelSettings />);

    // Wait for the async channel load to finish.
    await screen.findByText('My Telegram');

    // No confirm dialog before clicking delete.
    expect(screen.queryByText(t.notifications.deleteChannelConfirm)).not.toBeInTheDocument();

    const deleteButtons = screen.getAllByTitle(t.common.delete);
    await user.click(deleteButtons[0]);

    // The shared ConfirmDialog should now be visible with the i18n message.
    expect(await screen.findByText(t.notifications.deleteChannelConfirm)).toBeInTheDocument();
  });

  it('calls delete_notification_channel with the id when confirmed', async () => {
    const user = userEvent.setup();
    render(<ChannelSettings />);
    await screen.findByText('My Telegram');

    const deleteButtons = screen.getAllByTitle(t.common.delete);
    await user.click(deleteButtons[0]);

    await screen.findByText(t.notifications.deleteChannelConfirm);

    // Click the confirm button inside the dialog.
    const confirmButton = screen.getByRole('button', { name: t.common.confirm });
    await user.click(confirmButton);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('delete_notification_channel', { id: 1 });
    });
  });

  it('does NOT call delete_notification_channel when cancelled', async () => {
    const user = userEvent.setup();
    render(<ChannelSettings />);
    await screen.findByText('My Telegram');

    const deleteButtons = screen.getAllByTitle(t.common.delete);
    await user.click(deleteButtons[0]);

    await screen.findByText(t.notifications.deleteChannelConfirm);

    // Click the cancel button inside the dialog.
    const cancelButton = screen.getByRole('button', { name: t.common.cancel });
    await user.click(cancelButton);

    // The dialog should close and no delete invoke should be issued.
    await waitFor(() => {
      expect(screen.queryByText(t.notifications.deleteChannelConfirm)).not.toBeInTheDocument();
    });
    expect(invokeMock).not.toHaveBeenCalledWith('delete_notification_channel', expect.anything());
  });
});

describe('ChannelSettings — i18n', () => {
  it('renders channel-settings text in the active locale and updates on locale switch', async () => {
    const { rerender } = render(<ChannelSettings />);
    await screen.findByText('My Telegram');

    // zh_TW (default) strings should be present.
    const zhHeader = t.notifications.channels_label; // '通知通道'
    const zhAddChannel = t.notifications.addChannel;  // '新增通道'
    expect(screen.getByRole('heading', { name: zhHeader })).toBeInTheDocument();
    expect(screen.getByText(`+ ${zhAddChannel}`)).toBeInTheDocument();

    // Switch to English and re-render so the component reads the new `t`.
    await act(async () => { await setLocale('en'); });
    rerender(<ChannelSettings />);

    const enHeader = t.notifications.channels_label; // 'Notification Channels'
    const enAddChannel = t.notifications.addChannel;  // 'Add Channel'

    // The text must actually change with the locale (proves it comes from i18n).
    expect(enHeader).not.toBe(zhHeader);
    expect(screen.getByRole('heading', { name: enHeader })).toBeInTheDocument();
    expect(screen.getByText(`+ ${enAddChannel}`)).toBeInTheDocument();

    // The old localised header should no longer be present.
    expect(screen.queryByRole('heading', { name: zhHeader })).not.toBeInTheDocument();
  });
});
