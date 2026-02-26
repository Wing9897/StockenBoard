import { useState, useCallback, useRef } from 'react';

interface ConfirmState {
  message: string;
}

/**
 * 自訂 confirm dialog hook — 取代原生 confirm()
 * 回傳 { confirmState, requestConfirm, handleConfirm, handleCancel }
 */
export function useConfirm() {
  const [confirmState, setConfirmState] = useState<ConfirmState | null>(null);
  const resolveRef = useRef<((value: boolean) => void) | null>(null);

  const requestConfirm = useCallback((message: string): Promise<boolean> => {
    return new Promise(resolve => {
      resolveRef.current = resolve;
      setConfirmState({ message });
    });
  }, []);

  const handleConfirm = useCallback(() => {
    resolveRef.current?.(true);
    resolveRef.current = null;
    setConfirmState(null);
  }, []);

  const handleCancel = useCallback(() => {
    resolveRef.current?.(false);
    resolveRef.current = null;
    setConfirmState(null);
  }, []);

  return { confirmState, requestConfirm, handleConfirm, handleCancel };
}
