import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ConfirmDialog } from './ConfirmDialog';

describe('ConfirmDialog enhancements', () => {
  const defaultProps = {
    message: 'Are you sure?',
    onConfirm: vi.fn(),
    onCancel: vi.fn(),
  };

  it('renders an <h3> title when title prop is provided', () => {
    render(<ConfirmDialog {...defaultProps} title="Confirm Action" />);
    const heading = screen.getByRole('heading', { level: 3 });
    expect(heading).toBeInTheDocument();
    expect(heading).toHaveTextContent('Confirm Action');
  });

  it('does not render an <h3> title when title prop is omitted', () => {
    const { container } = render(<ConfirmDialog {...defaultProps} />);
    const heading = container.querySelector('h3');
    expect(heading).toBeNull();
  });

  it('confirm button has aria-label attribute', () => {
    render(<ConfirmDialog {...defaultProps} confirmLabel="Enable recording" />);
    const confirmBtn = screen.getByRole('button', { name: 'Enable recording' });
    expect(confirmBtn).toHaveAttribute('aria-label', 'Enable recording');
  });

  it('cancel button has aria-label attribute', () => {
    render(<ConfirmDialog {...defaultProps} cancelLabel="Dismiss dialog" />);
    const cancelBtn = screen.getByRole('button', { name: 'Dismiss dialog' });
    expect(cancelBtn).toHaveAttribute('aria-label', 'Dismiss dialog');
  });

  it('confirm button uses default aria-label from i18n when confirmLabel is not provided', () => {
    render(<ConfirmDialog {...defaultProps} />);
    // The confirm button should have an aria-label attribute (from t.common.confirm)
    const buttons = screen.getAllByRole('button');
    const confirmBtn = buttons.find(btn => btn.classList.contains('confirm'));
    expect(confirmBtn).toBeDefined();
    expect(confirmBtn!).toHaveAttribute('aria-label');
    expect(confirmBtn!.getAttribute('aria-label')).not.toBe('');
  });

  it('cancel button uses default aria-label from i18n when cancelLabel is not provided', () => {
    render(<ConfirmDialog {...defaultProps} />);
    const buttons = screen.getAllByRole('button');
    const cancelBtn = buttons.find(btn => btn.classList.contains('cancel'));
    expect(cancelBtn).toBeDefined();
    expect(cancelBtn!).toHaveAttribute('aria-label');
    expect(cancelBtn!.getAttribute('aria-label')).not.toBe('');
  });

  it('confirm button receives focus on open (autoFocus)', () => {
    render(<ConfirmDialog {...defaultProps} confirmLabel="Confirm" />);
    const confirmBtn = screen.getByRole('button', { name: 'Confirm' });
    expect(confirmBtn).toHaveFocus();
  });
});
