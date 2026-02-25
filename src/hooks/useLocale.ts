import { useState, useEffect } from 'react';
import { t as getT, getLocale, type LocaleId } from '../lib/i18n';

/** 訂閱語言變更，回傳最新的 t — 讓元件在切換語言時重新渲染 */
export function useLocale() {
  const [, setTick] = useState(0);
  useEffect(() => {
    const handler = () => setTick(v => v + 1);
    window.addEventListener('locale-change', handler);
    return () => window.removeEventListener('locale-change', handler);
  }, []);
  // 每次 render 都讀取最新的 module-level t
  return { t: getT, locale: getLocale() as LocaleId };
}
