/**
 * Provider 頁面共用常量 — Grid / List / Modal 共用
 */
import { t } from '../../lib/i18n';

export const TYPE_COLORS: Record<string, string> = {
  crypto: 'var(--peach)', stock: 'var(--blue)', both: 'var(--mauve)',
  prediction: 'var(--teal)', dex: 'var(--yellow)',
};

export const TYPE_BG: Record<string, string> = {
  crypto: 'var(--peach-bg)', stock: 'var(--blue-bg)', both: 'var(--mauve-bg)',
  prediction: 'var(--teal-bg)', dex: 'var(--yellow-bg)',
};

export function getTypeLabels(): Record<string, string> {
  return {
    crypto: t.providers.crypto, stock: t.providers.stock, both: t.providers.both,
    prediction: t.providers.prediction, dex: t.providers.dex,
  };
}

export function getTypeFilters() {
  return [
    { key: 'all', label: t.providers.all },
    { key: 'crypto', label: t.providers.crypto },
    { key: 'stock', label: t.providers.stock },
    { key: 'both', label: t.providers.both },
    { key: 'dex', label: t.providers.dex },
    { key: 'prediction', label: t.providers.prediction },
  ];
}
