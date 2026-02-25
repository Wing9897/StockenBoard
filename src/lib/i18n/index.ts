/**
 * 多語言系統 — 支援動態切換
 * 使用方式: import { t, setLocale, getLocale, LOCALES } from '../lib/i18n';
 */
import zh_TW_data from './zh_TW';
import zh_CN_data from './zh_CN';
import en_data from './en';
import ja_data from './ja';
import ko_data from './ko';

// ── Locale 定義 ──

export type LocaleId = 'zh_TW' | 'zh_CN' | 'en' | 'ja' | 'ko';
export type Locale = typeof zh_TW_data;

export const LOCALES: { id: LocaleId; label: string; flag: string }[] = [
  { id: 'zh_TW', label: '繁體中文', flag: '🇹🇼' },
  { id: 'zh_CN', label: '简体中文', flag: '🇨🇳' },
  { id: 'en', label: 'English', flag: '🇺🇸' },
  { id: 'ja', label: '日本語', flag: '🇯🇵' },
  { id: 'ko', label: '한국어', flag: '🇰🇷' },
];

// ── Locale Map & Reactive Switching ──

const localeMap: Record<LocaleId, Locale> = {
  zh_TW: zh_TW_data,
  zh_CN: zh_CN_data,
  en: en_data,
  ja: ja_data,
  ko: ko_data,
};

let _current: LocaleId = (localStorage.getItem('sb_locale') as LocaleId) || 'zh_TW';
if (!localeMap[_current]) _current = 'zh_TW';

/** 取得目前語言的所有字串 */
export let t: Locale = localeMap[_current];

/** 取得目前語言 ID */
export function getLocale(): LocaleId { return _current; }

/** 切換語言 — 觸發 'locale-change' 事件讓 React 元件重新渲染 */
export function setLocale(id: LocaleId) {
  if (!localeMap[id]) return;
  _current = id;
  t = localeMap[id];
  localStorage.setItem('sb_locale', id);
  window.dispatchEvent(new Event('locale-change'));
}
