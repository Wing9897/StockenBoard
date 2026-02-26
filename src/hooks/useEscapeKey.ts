import { useEffect } from 'react';

/** 當按下 Escape 時呼叫 callback，自動清理 listener */
export function useEscapeKey(callback: () => void) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') callback(); };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [callback]);
}
