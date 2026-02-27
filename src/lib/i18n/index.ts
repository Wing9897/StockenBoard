/**
 * 多語言系統 — 支援動態切換 + 懶載入
 * 使用方式: import { t, setLocale, getLocale, LOCALES } from '../lib/i18n';
 */
import zh_TW_data from './zh_TW';

export type LocaleId = 'zh_TW' | 'zh_CN' | 'en' | 'ja' | 'ko';
export type Locale = typeof zh_TW_data;

export const LOCALES: { id: LocaleId; label: string; flag: string }[] = [
  { id: 'zh_TW', label: '繁體中文', flag: '🇹🇼' },
  { id: 'zh_CN', label: '简体中文', flag: '🇨🇳' },
  { id: 'en', label: 'English', flag: '🇺🇸' },
  { id: 'ja', label: '日本語', flag: '🇯🇵' },
  { id: 'ko', label: '한국어', flag: '🇰🇷' },
];

// 懶載入 — 只有 zh_TW 是同步載入（預設語言），其他語言按需載入
const loaders: Record<LocaleId, () => Promise<Locale>> = {
  zh_TW: () => Promise.resolve(zh_TW_data),
  zh_CN: () => import('./zh_CN').then(m => m.default),
  en: () => import('./en').then(m => m.default),
  ja: () => import('./ja').then(m => m.default),
  ko: () => import('./ko').then(m => m.default),
};

const loaded: Partial<Record<LocaleId, Locale>> = { zh_TW: zh_TW_data };

let _current: LocaleId = (localStorage.getItem('sb_locale') as LocaleId) || 'zh_TW';
if (!loaders[_current]) _current = 'zh_TW';

/** 取得目前語言的所有字串 */
export let t: Locale = loaded[_current] || zh_TW_data;

/** 取得目前語言 ID */
export function getLocale(): LocaleId { return _current; }

/** 切換語言 — 觸發 'locale-change' 事件讓 React 元件重新渲染 */
export async function setLocale(id: LocaleId) {
  if (!loaders[id]) return;
  if (!loaded[id]) loaded[id] = await loaders[id]();
  _current = id;
  t = loaded[id]!;
  localStorage.setItem('sb_locale', id);
  window.dispatchEvent(new Event('locale-change'));
}

// 啟動時載入已儲存的語言（非 zh_TW 時）
if (_current !== 'zh_TW') {
  loaders[_current]().then(data => {
    loaded[_current] = data;
    t = data;
    window.dispatchEvent(new Event('locale-change'));
  });
}
