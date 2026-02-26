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

/** 截斷地址顯示 — DexCard / DexEditPanel 共用 */
export function truncateAddr(addr: string, len = 6): string {
  if (!addr) return '-';
  if (addr.length <= len * 2 + 2) return addr;
  return `${addr.slice(0, len)}...${addr.slice(-4)}`;
}

/** 從 displayName 解析交易對 — DexCard 共用 */
export function parsePairFromName(displayName: string | undefined): [string, string] {
  const dn = displayName || '';
  const sep = dn.includes('/') ? '/' : dn.includes('→') ? '→' : null;
  if (sep) {
    const parts = dn.split(sep).map(s => s.trim());
    if (parts.length === 2 && parts[0] && parts[1]) return [parts[0], parts[1]];
  }
  return ['', ''];
}

/** 從冗長的批量錯誤訊息中提取簡短摘要
 *  注意：regex 匹配的是 Rust 後端回傳的中文錯誤格式，這是刻意設計 */
export function summarizeError(error: string): string {
  const deduped = error.match(/批量查詢全部失敗\s*\(\d+個\):\s*(.+)/);
  if (deduped) return deduped[1].trim();
  const batchMatch = error.match(/批量查詢全部失敗:\s*\w[^:]*:\s*(.+?)(?::\s*error\b|;\s*\w|$)/);
  if (batchMatch) return batchMatch[1].trim();
  if (/error sending request/i.test(error)) return t.errors.connectionFailed;
  if (error.length > 60) return error.slice(0, 57) + '...';
  return error;
}
