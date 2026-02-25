/**
 * 共用格式化工具 — AssetCard 和 DexCard 共用
 */
import { t } from './i18n';

export function formatNumber(num: number | undefined | null, decimals = 2): string {
  if (num === undefined || num === null) return '-';
  if (num >= 1e12) return (num / 1e12).toFixed(2) + 'T';
  if (num >= 1e9) return (num / 1e9).toFixed(2) + 'B';
  if (num >= 1e6) return (num / 1e6).toFixed(2) + 'M';
  if (num >= 1e3) return (num / 1e3).toFixed(2) + 'K';
  return num.toFixed(decimals);
}

export function formatPrice(price: number | undefined | null, currency: string = 'USD'): string {
  if (price === undefined || price === null || isNaN(price)) return '-';
  const sym = currency === 'USD' || currency === 'USDT' ? '$' : currency + ' ';
  if (price >= 1) return sym + price.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  return sym + price.toPrecision(4);
}

/** 從冗長的批量錯誤訊息中提取簡短摘要 */
export function summarizeError(error: string): string {
  const deduped = error.match(/批量查詢全部失敗\s*\(\d+個\):\s*(.+)/);
  if (deduped) return deduped[1].trim();
  const batchMatch = error.match(/批量查詢全部失敗:\s*\w[^:]*:\s*(.+?)(?::\s*error\b|;\s*\w|$)/);
  if (batchMatch) return batchMatch[1].trim();
  if (/error sending request/i.test(error)) return t.errors.connectionFailed;
  if (error.length > 60) return error.slice(0, 57) + '...';
  return error;
}
