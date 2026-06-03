import { describe, it, expect } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useConfirm } from './useConfirm';

// Characterization tests for the promise-based confirm hook that backs the
// app-wide ConfirmDialog (replaces native confirm()).

describe('useConfirm', () => {
  it('starts with no active confirm state', () => {
    const { result } = renderHook(() => useConfirm());
    expect(result.current.confirmState).toBeNull();
  });

  it('exposes the message while a confirm is pending', () => {
    const { result } = renderHook(() => useConfirm());
    act(() => {
      void result.current.requestConfirm('Delete this?');
    });
    expect(result.current.confirmState).toEqual({ message: 'Delete this?' });
  });

  it('resolves the promise with true and clears state on confirm', async () => {
    const { result } = renderHook(() => useConfirm());
    let pending: Promise<boolean>;
    act(() => {
      pending = result.current.requestConfirm('Proceed?');
    });
    act(() => {
      result.current.handleConfirm();
    });
    await expect(pending!).resolves.toBe(true);
    expect(result.current.confirmState).toBeNull();
  });

  it('resolves the promise with false and clears state on cancel', async () => {
    const { result } = renderHook(() => useConfirm());
    let pending: Promise<boolean>;
    act(() => {
      pending = result.current.requestConfirm('Proceed?');
    });
    act(() => {
      result.current.handleCancel();
    });
    await expect(pending!).resolves.toBe(false);
    expect(result.current.confirmState).toBeNull();
  });
});
