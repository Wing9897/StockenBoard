import { useState, useCallback, useRef } from 'react';
import { ToastType, ToastMessage } from '../components/Toast/Toast';

export function useToast() {
  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  const idRef = useRef(0);

  const addToast = useCallback((type: ToastType, title: string, message?: string, duration?: number) => {
    const id = ++idRef.current;
    setToasts((prev) => [...prev.slice(-4), { id, type, title, message, duration }]);
  }, []);

  const removeToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const success = useCallback((title: string, message?: string) => addToast('success', title, message), [addToast]);
  const error = useCallback((title: string, message?: string) => addToast('error', title, message, 6000), [addToast]);
  const info = useCallback((title: string, message?: string) => addToast('info', title, message), [addToast]);
  const warning = useCallback((title: string, message?: string) => addToast('warning', title, message, 5000), [addToast]);

  return { toasts, removeToast, success, error, info, warning };
}
