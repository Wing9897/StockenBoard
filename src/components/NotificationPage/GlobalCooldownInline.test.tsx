import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';

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

import { t } from '../../lib/i18n';
import { GlobalCooldownInline } from './GlobalCooldownInline';

beforeEach(() => {
  vi.useFakeTimers();
  mockInvoke.mockReset();
  // Default: return 30 for get_notification_global_cooldown
  mockInvoke.mockImplementation((cmd: string) => {
    if (cmd === 'get_notification_global_cooldown') return Promise.resolve(30);
    if (cmd === 'set_notification_global_cooldown') return Promise.resolve();
    return Promise.resolve();
  });
});

afterEach(() => {
  vi.useRealTimers();
});

describe('GlobalCooldownInline', () => {
  it('renders input[type="number"] with min=0, max=3600, step=1', async () => {
    await act(async () => {
      render(<GlobalCooldownInline />);
    });
    // Flush the loading promise
    await act(async () => {
      await vi.runAllTimersAsync();
    });

    const input = screen.getByRole('spinbutton') as HTMLInputElement;
    expect(input).toBeInTheDocument();
    expect(input.type).toBe('number');
    expect(input.min).toBe('0');
    expect(input.max).toBe('3600');
    expect(input.step).toBe('1');
  });

  it('renders inside a .global-cooldown-inline container', async () => {
    await act(async () => {
      render(<GlobalCooldownInline />);
    });
    await act(async () => {
      await vi.runAllTimersAsync();
    });

    const input = screen.getByRole('spinbutton');
    expect(input.closest('.global-cooldown-inline')).not.toBeNull();
  });

  it('displays the unit label', async () => {
    await act(async () => {
      render(<GlobalCooldownInline />);
    });
    await act(async () => {
      await vi.runAllTimersAsync();
    });

    expect(screen.getByText(t.notifications.globalCooldownUnit)).toBeInTheDocument();
  });

  it('debounces save at 300ms after value change', async () => {
    await act(async () => {
      render(<GlobalCooldownInline />);
    });
    await act(async () => {
      await vi.runAllTimersAsync();
    });

    const input = screen.getByRole('spinbutton') as HTMLInputElement;

    // Change value to 60
    await act(async () => {
      fireEvent.change(input, { target: { value: '60' } });
    });

    // Before 300ms: save should not have been called
    expect(mockInvoke).not.toHaveBeenCalledWith('set_notification_global_cooldown', { secs: 60 });

    // Advance time by 299ms — still should not save
    await act(async () => {
      vi.advanceTimersByTime(299);
    });
    expect(mockInvoke).not.toHaveBeenCalledWith('set_notification_global_cooldown', { secs: 60 });

    // Advance time by 1ms more (total 300ms) — should save now
    await act(async () => {
      vi.advanceTimersByTime(1);
    });
    expect(mockInvoke).toHaveBeenCalledWith('set_notification_global_cooldown', { secs: 60 });
  });
});
