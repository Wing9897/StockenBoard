/**
 * 訂閱 CRUD 操作 — 純 data layer，不依賴 React。
 * 從 useAssetData.ts 抽出，讓業務邏輯可獨立測試。
 */
import { invoke } from '@tauri-apps/api/core';
import type { Subscription, ProviderInfo } from '../types';
import { getDb } from './db';
import { silentLog } from './errorLog';

/** 載入指定類型的訂閱列表 */
export async function loadSubscriptions(subType: 'asset' | 'dex'): Promise<Subscription[]> {
  const db = await getDb();
  return db.select<Subscription[]>(
    'SELECT id, sub_type, symbol, display_name, selected_provider_id, asset_type, pool_address, token_from_address, token_to_address, sort_order, record_enabled, record_from_hour, record_to_hour FROM subscriptions WHERE sub_type = $1 ORDER BY sort_order, id',
    [subType]
  );
}

/** 載入所有 provider 資訊 */
export async function loadProviderInfo(): Promise<ProviderInfo[]> {
  return invoke<ProviderInfo[]>('get_all_providers');
}

/** 通知 Rust 後端重新載入 polling */
export async function reloadPolling(): Promise<void> {
  await invoke('reload_polling');
}

/** 判斷 provider 是否為 DEX 類型 */
export function isDexProvider(providers: ProviderInfo[], providerId: string): boolean {
  return providers.find(p => p.id === providerId)?.provider_type === 'dex';
}

/** 新增 asset 訂閱（先驗證 symbol 有效性） */
export async function addAssetSubscription(
  symbol: string, providers: ProviderInfo[],
  displayName?: string, providerId?: string, assetType?: string,
): Promise<void> {
  const pid = providerId || 'binance';
  const isDex = isDexProvider(providers, pid);
  const storedSymbol = isDex ? symbol.trim() : symbol.toUpperCase();
  await invoke('fetch_asset_price', { providerId: pid, symbol: storedSymbol });
  const db = await getDb();
  await db.execute(
    'INSERT INTO subscriptions (sub_type, symbol, display_name, selected_provider_id, asset_type) VALUES ($1, $2, $3, $4, $5)',
    ['asset', storedSymbol, displayName || null, pid, assetType || 'crypto']
  );
}

/** 批量新增 asset 訂閱（使用批量 API 一次驗證，避免大量並行請求卡住） */
export async function addAssetSubscriptionBatch(
  items: { symbol: string; displayName?: string; providerId?: string; assetType?: string }[],
  providers: ProviderInfo[],
  onProgress?: (done: number, total: number) => void,
): Promise<{ succeeded: string[]; failed: string[]; dbDuplicates: string[] }> {
  const total = items.length;
  type PreparedItem = { symbol: string; displayName?: string; assetType?: string; storedSymbol: string; pid: string };

  // 1. 按 provider 分組，並預處理 symbol
  const groups = new Map<string, PreparedItem[]>();
  for (const item of items) {
    const pid = item.providerId || 'binance';
    const isDex = isDexProvider(providers, pid);
    const storedSymbol = isDex ? item.symbol.trim() : item.symbol.toUpperCase();
    const prepared: PreparedItem = { symbol: item.symbol, displayName: item.displayName, assetType: item.assetType, storedSymbol, pid };
    const list = groups.get(pid);
    if (list) list.push(prepared);
    else groups.set(pid, [prepared]);
  }

  // 2. 每個 provider 用一次批量 API 驗證所有 symbol（而非逐個發請求）
  const allValid: PreparedItem[] = [];
  const failed: string[] = [];
  let done = 0;

  for (const [pid, group] of groups) {
    const symbols = group.map(g => g.storedSymbol);
    try {
      // fetch_multiple_prices 回傳成功取得價格的 AssetData[]
      // Binance 的實作用單一 HTTP 請求 /api/v3/ticker/24hr?symbols=[...]
      const results = await invoke<{ symbol: string }[]>('fetch_multiple_prices', { providerId: pid, symbols });
      const validSymbols = new Set(results.map(r => r.symbol.toUpperCase()));
      for (const g of group) {
        if (validSymbols.has(g.storedSymbol.toUpperCase())) {
          allValid.push(g);
        } else {
          failed.push(g.symbol);
        }
      }
    } catch {
      // 整個批量請求失敗 → 該 provider 所有 symbol 都失敗
      for (const g of group) failed.push(g.symbol);
    }
    done += group.length;
    onProgress?.(done, total);
  }

  // 3. 寫入 DB
  const succeeded: string[] = [];
  const dbDuplicates: string[] = [];
  if (allValid.length > 0) {
    const db = await getDb();
    await db.execute('BEGIN TRANSACTION');
    for (const v of allValid) {
      try {
        await db.execute(
          'INSERT INTO subscriptions (sub_type, symbol, display_name, selected_provider_id, asset_type) VALUES ($1, $2, $3, $4, $5)',
          ['asset', v.storedSymbol, v.displayName || null, v.pid, v.assetType || 'crypto']
        );
        succeeded.push(v.storedSymbol);
      } catch {
        dbDuplicates.push(v.storedSymbol);
      }
    }
    await db.execute('COMMIT');
  }
  return { succeeded, failed, dbDuplicates };
}

/** 新增 DEX 訂閱 */
export async function addDexSubscription(
  poolAddress: string, tokenFrom: string, tokenTo: string,
  providerId: string, displayName?: string,
): Promise<void> {
  const db = await getDb();
  const symbol = `${poolAddress.trim()}:${tokenFrom.trim()}:${tokenTo.trim()}`;
  await db.execute(
    'INSERT INTO subscriptions (sub_type, symbol, display_name, selected_provider_id, asset_type, pool_address, token_from_address, token_to_address) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)',
    ['dex', symbol, displayName || null, providerId, 'crypto', poolAddress.trim(), tokenFrom.trim(), tokenTo.trim()]
  );
}

/** 更新 asset 訂閱 */
export async function updateAssetSubscription(
  sub: Subscription, providers: ProviderInfo[],
  updates: { symbol?: string; displayName?: string; providerId?: string; assetType?: string },
): Promise<boolean> {
  const db = await getDb();
  const targetPid = updates.providerId ?? sub.selected_provider_id;
  const isDex = isDexProvider(providers, targetPid);
  const storedSymbol = updates.symbol ? (isDex ? updates.symbol.trim() : updates.symbol.toUpperCase()) : sub.symbol;
  await db.execute(
    'UPDATE subscriptions SET symbol = $1, display_name = $2, selected_provider_id = $3, asset_type = $4 WHERE id = $5',
    [storedSymbol, updates.displayName !== undefined ? (updates.displayName || null) : (sub.display_name || null), targetPid, updates.assetType ?? sub.asset_type, sub.id]
  );
  return !!(updates.symbol || updates.providerId); // 回傳是否需要 reload polling
}

/** 更新 DEX 訂閱 */
export async function updateDexSub(
  sub: Subscription,
  updates: { poolAddress?: string; tokenFrom?: string; tokenTo?: string; providerId?: string; displayName?: string },
): Promise<void> {
  const db = await getDb();
  const pool = updates.poolAddress?.trim() ?? sub.pool_address ?? '';
  const tf = updates.tokenFrom?.trim() ?? sub.token_from_address ?? '';
  const tt = updates.tokenTo?.trim() ?? sub.token_to_address ?? '';
  const symbol = `${pool}:${tf}:${tt}`;
  await db.execute(
    'UPDATE subscriptions SET symbol = $1, pool_address = $2, token_from_address = $3, token_to_address = $4, selected_provider_id = $5, display_name = $6 WHERE id = $7',
    [symbol, pool, tf, tt, updates.providerId ?? sub.selected_provider_id, updates.displayName !== undefined ? (updates.displayName || null) : (sub.display_name || null), sub.id]
  );
}

/** 檢查 provider 是否已設定 API key */
export async function hasApiKey(providerId: string): Promise<boolean> {
  try {
    const db = await getDb();
    const rows = await db.select<{ api_key: string | null }[]>(
      'SELECT api_key FROM provider_settings WHERE provider_id = $1',
      [providerId]
    );
    return rows.length > 0 && !!rows[0].api_key;
  } catch (e) { silentLog('hasApiKey', e); return false; }
}

/** 儲存 provider API key（含同步 Rust 端） */
export async function saveApiKey(providerId: string, apiKey: string, apiSecret?: string): Promise<void> {
  const db = await getDb();
  await db.execute(
    `INSERT INTO provider_settings (provider_id, api_key, api_secret, connection_type)
     VALUES ($1, $2, $3, 'rest')
     ON CONFLICT(provider_id) DO UPDATE SET api_key = $2, api_secret = $3`,
    [providerId, apiKey || null, apiSecret || null]
  );
  await invoke('enable_provider', {
    providerId,
    apiKey: apiKey || null,
    apiSecret: apiSecret || null,
  });
}

/** 刪除單一訂閱 */
export async function removeSubscription(id: number): Promise<void> {
  const db = await getDb();
  await db.execute('DELETE FROM subscriptions WHERE id = $1', [id]);
}

/** 批量刪除訂閱 */
export async function removeSubscriptions(ids: number[]): Promise<void> {
  if (ids.length === 0) return;
  const db = await getDb();
  const placeholders = ids.map((_, i) => '$' + (i + 1)).join(',');
  await db.execute(`DELETE FROM subscriptions WHERE id IN (${placeholders})`, ids);
}
