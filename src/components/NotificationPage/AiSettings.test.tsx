import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor, act, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// Mock the transport layer.
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

import { AiSettings } from './AiSettings';
import { t, setLocale } from '../../lib/i18n';

/** Default config returned by get_ai_provider_config */
const DEFAULT_CONFIG = {
  base_url: 'http://localhost:11434/v1',
  model: 'llama3.1:8b',
  has_api_key: false,
  disable_thinking: true,
  max_context_tokens: null,
};

function defaultInvoke(cmd: string): Promise<unknown> {
  if (cmd === 'get_ai_provider_config') return Promise.resolve(DEFAULT_CONFIG);
  if (cmd === 'list_ai_models') return Promise.resolve(['llama3.1:8b']);
  return Promise.resolve();
}

beforeEach(async () => {
  invokeMock.mockReset();
  invokeMock.mockImplementation((cmd: string) => defaultInvoke(cmd));
  await act(async () => { await setLocale('zh_TW'); });
});

afterEach(() => {
  cleanup();
});

/** Helper: find the max_context_tokens input by its associated label text */
function getMaxTokensInput(container: HTMLElement): HTMLInputElement {
  // The label wraps both the <span> with the label text and the <input>
  const span = Array.from(container.querySelectorAll('span')).find(
    (el) => el.textContent === t.notifications.maxContextTokensLabel
  );
  if (!span) throw new Error(`Could not find span with text: ${t.notifications.maxContextTokensLabel}`);
  const label = span.closest('label');
  if (!label) throw new Error('Could not find parent <label>');
  const input = label.querySelector('input');
  if (!input) throw new Error('Could not find <input> inside label');
  return input as HTMLInputElement;
}

describe('AiSettings — max_context_tokens field', () => {
  it('renders the max_context_tokens input field', async () => {
    const { container } = render(<AiSettings />);

    // Wait for loading to complete
    await waitFor(() => {
      expect(screen.queryByText(t.common.loading)).not.toBeInTheDocument();
    });

    const input = getMaxTokensInput(container);
    expect(input).toBeInTheDocument();
    expect(input.type).toBe('number');
  });

  it('validation rejects values below 500 and shows error message', async () => {
    const user = userEvent.setup();
    const { container } = render(<AiSettings />);

    await waitFor(() => {
      expect(screen.queryByText(t.common.loading)).not.toBeInTheDocument();
    });

    const input = getMaxTokensInput(container);

    // Type a value below 500
    await user.clear(input);
    await user.type(input, '499');

    // Error message should be displayed
    expect(screen.getByText(t.notifications.minContextTokensError)).toBeInTheDocument();
  });

  it('empty field is valid (unconfigured) — no error shown', async () => {
    const user = userEvent.setup();
    const { container } = render(<AiSettings />);

    await waitFor(() => {
      expect(screen.queryByText(t.common.loading)).not.toBeInTheDocument();
    });

    const input = getMaxTokensInput(container);

    // Type a value, then clear it
    await user.clear(input);
    await user.type(input, '100');
    // Error should appear for 100
    expect(screen.getByText(t.notifications.minContextTokensError)).toBeInTheDocument();

    // Now clear the field
    await user.clear(input);

    // Error should disappear — empty is valid
    expect(screen.queryByText(t.notifications.minContextTokensError)).not.toBeInTheDocument();
  });

  it('save includes max_context_tokens in payload when a valid value is entered', async () => {
    const user = userEvent.setup();
    const { container } = render(<AiSettings />);

    await waitFor(() => {
      expect(screen.queryByText(t.common.loading)).not.toBeInTheDocument();
    });

    const input = getMaxTokensInput(container);
    await user.clear(input);
    await user.type(input, '4096');

    // Submit the form
    const saveButton = screen.getByRole('button', { name: t.common.save });
    await user.click(saveButton);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('save_ai_provider_config', expect.objectContaining({
        max_context_tokens: 4096,
      }));
    });
  });

  it('save sends null when max_context_tokens field is empty', async () => {
    const user = userEvent.setup();
    const { container } = render(<AiSettings />);

    await waitFor(() => {
      expect(screen.queryByText(t.common.loading)).not.toBeInTheDocument();
    });

    const input = getMaxTokensInput(container);
    // Ensure the field is empty
    await user.clear(input);

    // Submit the form
    const saveButton = screen.getByRole('button', { name: t.common.save });
    await user.click(saveButton);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('save_ai_provider_config', expect.objectContaining({
        max_context_tokens: null,
      }));
    });
  });
});
