/**
 * 共用格式化工具 — AssetCard、DexCard、AlertSidebar、NotificationPage 共用
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

/** 從冗長的批量錯誤訊息中提取簡短摘要 */
export function summarizeError(error: string): string {
  if (/error sending request/i.test(error)) return t.errors.connectionFailed;
  if (/batch.*failed/i.test(error)) {
    // Extract the core message after the provider prefix
    const core = error.replace(/^\w[\w.]*\s+batch\s+\w+\s+failed:\s*/i, '');
    return core.length > 60 ? core.slice(0, 57) + '...' : core;
  }
  if (error.length > 60) return error.slice(0, 57) + '...';
  return error;
}

/** Unix 時間戳（秒）轉本地時間字串 — HistoryTable / AlertSidebar 共用 */
export function formatTimestamp(unix: number): string {
  return new Date(unix * 1000).toLocaleString();
}

/** 格式化通知條件標籤 — RuleList / AlertSidebar 共用 */
export function formatConditionLabel(conditionType: string, threshold: number): string {
  switch (conditionType) {
    case 'price_above': return t.notifications.condPriceAbove(threshold.toLocaleString());
    case 'price_below': return t.notifications.condPriceBelow(threshold.toLocaleString());
    case 'change_pct_above': return t.notifications.condChangeUp(String(threshold));
    case 'change_pct_below': return t.notifications.condChangeDown(String(threshold));
    case 'ai': return t.notifications.aiRule;
    default: return conditionType;
  }
}

/** 本地時區標籤（e.g. "UTC+8"）— HistoryPage / ProviderModal 共用 */
export const TZ_LABEL = (() => {
  const off = -new Date().getTimezoneOffset();
  const h = Math.floor(Math.abs(off) / 60);
  const m = Math.abs(off) % 60;
  return `UTC${off >= 0 ? '+' : '-'}${h}${m ? ':' + String(m).padStart(2, '0') : ''}`;
})();
