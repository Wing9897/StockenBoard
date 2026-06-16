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

import { AlertSidebar } from './AlertSidebar';

beforeEach(() => {
  mockInvoke.mockReset();
  mockInvoke.mockImplementation((cmd: string) => {
    switch (cmd) {
      case 'get_notification_history':
        return Promise.resolve([
          {
            id: 1,
            rule_id: 1,
            channel_id: 1,
            status: 'sent',
            price: 100,
            message: 'BTC price above 50000',
            error: null,
            sent_at: 1700000000,
          },
          {
            id: 2,
            rule_id: 2,
            channel_id: 1,
            status: 'sent',
            price: 200,
            message: 'ETH change pct down 5%',
            error: null,
            sent_at: 1700000100,
          },
        ]);
      default:
        return Promise.resolve();
    }
  });
});

describe('FilterBar renders search input and condition dropdown', () => {
  it('renders search input with placeholder "Search alerts..."', async () => {
    await act(async () => {
      render(<AlertSidebar panelOpen={true} onClose={() => {}} />);
    });

    const searchInput = screen.getByPlaceholderText('Search alerts...');
    expect(searchInput).toBeInTheDocument();
    expect(searchInput).toHaveAttribute('type', 'text');
  });

  it('renders condition dropdown with 6 options', async () => {
    await act(async () => {
      render(<AlertSidebar panelOpen={true} onClose={() => {}} />);
    });

    const select = screen.getByRole('combobox');
    expect(select).toBeInTheDocument();

    const options = select.querySelectorAll('option');
    expect(options).toHaveLength(6);
    expect(options[0]).toHaveTextContent('All');
    expect(options[1]).toHaveTextContent('Price Above');
    expect(options[2]).toHaveTextContent('Price Below');
    expect(options[3]).toHaveTextContent('Change % Up');
    expect(options[4]).toHaveTextContent('Change % Down');
    expect(options[5]).toHaveTextContent('AI');
  });
});

describe('FilterBar clear button resets both filters', () => {
  it('shows clear button when search text is entered and resets on click', async () => {
    await act(async () => {
      render(<AlertSidebar panelOpen={true} onClose={() => {}} />);
    });

    const searchInput = screen.getByPlaceholderText('Search alerts...');
    const select = screen.getByRole('combobox');

    // Initially no clear button
    expect(screen.queryByLabelText('Clear filters')).not.toBeInTheDocument();

    // Set search text
    await act(async () => {
      fireEvent.change(searchInput, { target: { value: 'BTC' } });
    });

    // Clear button should now appear
    const clearBtn = screen.getByLabelText('Clear filters');
    expect(clearBtn).toBeInTheDocument();

    // Change condition filter too
    await act(async () => {
      fireEvent.change(select, { target: { value: 'price_above' } });
    });

    // Click clear button
    await act(async () => {
      fireEvent.click(clearBtn);
    });

    // Both should reset
    expect(searchInput).toHaveValue('');
    expect(select).toHaveValue('all');

    // Clear button should disappear
    expect(screen.queryByLabelText('Clear filters')).not.toBeInTheDocument();
  });
});

describe('Empty state message when filter returns no results', () => {
  it('displays "無符合條件的通知" when items exist but filter matches nothing', async () => {
    await act(async () => {
      render(<AlertSidebar panelOpen={true} onClose={() => {}} />);
    });

    // Wait for history to load
    await act(async () => {
      await new Promise(r => setTimeout(r, 50));
    });

    const searchInput = screen.getByPlaceholderText('Search alerts...');

    // Type a search that matches nothing
    await act(async () => {
      fireEvent.change(searchInput, { target: { value: 'zzzznonexistent' } });
    });

    // Advance past debounce (200ms)
    await act(async () => {
      await new Promise(r => setTimeout(r, 250));
    });

    expect(screen.getByText('無符合條件的通知')).toBeInTheDocument();
  });
});

describe('Search debounce at 200ms', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('does not apply filter immediately, applies after 200ms', async () => {
    await act(async () => {
      render(<AlertSidebar panelOpen={true} onClose={() => {}} />);
    });

    // Wait for history to load
    await act(async () => {
      await vi.advanceTimersByTimeAsync(50);
    });

    // Items should be loaded (the messages are rendered as rule_name)
    // Verify items exist by checking no empty state
    expect(screen.queryByText('無符合條件的通知')).not.toBeInTheDocument();

    const searchInput = screen.getByPlaceholderText('Search alerts...');

    // Type a search that matches nothing
    await act(async () => {
      fireEvent.change(searchInput, { target: { value: 'zzzznonexistent' } });
    });

    // Before 200ms: debounce has not fired, empty state should NOT show yet
    await act(async () => {
      await vi.advanceTimersByTimeAsync(100);
    });
    expect(screen.queryByText('無符合條件的通知')).not.toBeInTheDocument();

    // After 200ms: debounce fires, empty state should show
    await act(async () => {
      await vi.advanceTimersByTimeAsync(150);
    });
    expect(screen.getByText('無符合條件的通知')).toBeInTheDocument();
  });
});
