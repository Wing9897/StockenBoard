/**
 * 訂閱 CRUD 操作 — 純 data layer，不依賴 React。
 * 全部走 Rust IPC，不再直接操作 SQL。
 */
import { invoke } from '@tauri-apps/api/core';
import type { Subscription, ProviderInfo } from '../types';
import { silentLog } from './errorLog';

/** 載入指定類型的訂閱列表 */
export async function loadSubscriptions(subType: 'asset' | 'dex'): Promise<Subscription[]> {
  return invoke<Subscription[]>('list_subscriptions', { subType });
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
  // 先驗證 symbol 有效性
  await invoke('fetch_asset_price', { providerId: pid, symbol: storedSymbol });
  // 再新增訂閱（走 Rust IPC）
  await invoke<number>('add_subscription', {
    subType: 'asset',
    symbol: storedSymbol,
    displayName: displayName || null,
    providerId: pid,
    assetType: assetType || 'crypto',
    poolAddress: null,
    tokenFrom: null,
    tokenTo: null,
  });
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

  // 2. 每個 provider 用一次批量 API 驗證所有 symbol
  const allValid: PreparedItem[] = [];
  const failed: string[] = [];
  let done = 0;

  for (const [pid, group] of groups) {
    const symbols = group.map(g => g.storedSymbol);
    try {
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
      for (const g of group) failed.push(g.symbol);
    }
    done += group.length;
    onProgress?.(done, total);
  }

  // 3. 批量寫入 DB（走 Rust IPC，一次搞定）
  const succeeded: string[] = [];
  const dbDuplicates: string[] = [];
  if (allValid.length > 0) {
    try {
      const batchItems = allValid.map(v => ({
        symbol: v.storedSymbol,
        display_name: v.displayName || null,
        provider_id: v.pid,
        asset_type: v.assetType || 'crypto',
      }));
      const result = await invoke<{ succeeded: string[]; failed: string[]; duplicates: string[] }>(
        'add_subscriptions_batch',
        { items: batchItems }
      );
      succeeded.push(...result.succeeded);
      dbDuplicates.push(...result.duplicates);
      failed.push(...result.failed);
    } catch (e) {
      silentLog('addSubscriptionBatch', e);
      // 如果批量失敗，逐個嘗試
      for (const v of allValid) {
        try {
          await invoke<number>('add_subscription', {
            subType: 'asset',
            symbol: v.storedSymbol,
            displayName: v.displayName || null,
            providerId: v.pid,
            assetType: v.assetType || 'crypto',
            poolAddress: null,
            tokenFrom: null,
            tokenTo: null,
          });
          succeeded.push(v.storedSymbol);
        } catch (e2) {
          const msg = String(e2);
          if (msg.includes('已存在')) {
            dbDuplicates.push(v.storedSymbol);
          } else {
            failed.push(v.symbol);
          }
        }
      }
    }
  }
  return { succeeded, failed, dbDuplicates };
}

/** 新增 DEX 訂閱 */
export async function addDexSubscription(
  poolAddress: string, tokenFrom: string, tokenTo: string,
  providerId: string, displayName?: string,
): Promise<void> {
  const pool = poolAddress.trim();
  const tf = tokenFrom.trim();
  const tt = tokenTo.trim();
  const symbol = `${pool}:${tf}:${tt}`;
  await invoke<number>('add_subscription', {
    subType: 'dex',
    symbol,
    displayName: displayName || null,
    providerId,
    assetType: 'crypto',
    poolAddress: pool,
    tokenFrom: tf,
    tokenTo: tt,
  });
}

/** 更新 asset 訂閱 */
export async function updateAssetSubscription(
  sub: Subscription, providers: ProviderInfo[],
  updates: { symbol?: string; displayName?: string; providerId?: string; assetType?: string },
): Promise<boolean> {
  const targetPid = updates.providerId ?? sub.selected_provider_id;
  const isDex = isDexProvider(providers, targetPid);
  const storedSymbol = updates.symbol ? (isDex ? updates.symbol.trim() : updates.symbol.toUpperCase()) : sub.symbol;
  await invoke('update_subscription', {
    id: sub.id,
    symbol: storedSymbol,
    displayName: updates.displayName !== undefined ? (updates.displayName || null) : (sub.display_name || null),
    providerId: targetPid,
    assetType: updates.assetType ?? sub.asset_type,
  });
  return !!(updates.symbol || updates.providerId);
}

/** 更新 DEX 訂閱 */
export async function updateDexSub(
  sub: Subscription,
  updates: { poolAddress?: string; tokenFrom?: string; tokenTo?: string; providerId?: string; displayName?: string },
): Promise<void> {
  const pool = updates.poolAddress?.trim() ?? sub.pool_address ?? '';
  const tf = updates.tokenFrom?.trim() ?? sub.token_from_address ?? '';
  const tt = updates.tokenTo?.trim() ?? sub.token_to_address ?? '';
  const symbol = `${pool}:${tf}:${tt}`;
  // For DEX updates, we update the full subscription including pool fields
  await invoke('update_subscription', {
    id: sub.id,
    symbol,
    displayName: updates.displayName !== undefined ? (updates.displayName || null) : (sub.display_name || null),
    providerId: updates.providerId ?? sub.selected_provider_id,
    assetType: sub.asset_type,
  });
}

/** 檢查 provider 是否已設定 API key */
export async function hasApiKey(providerId: string): Promise<boolean> {
  try {
    return await invoke<boolean>('has_api_key', { providerId });
  } catch (e) { silentLog('hasApiKey', e); return false; }
}

/** 儲存 provider 設定（含 API key + 同步 Rust 端） */
export async function saveApiKey(providerId: string, apiKey: string, apiSecret?: string): Promise<void> {
  await invoke('upsert_provider_settings', {
    providerId,
    apiKey: apiKey || null,
    apiSecret: apiSecret || null,
    apiUrl: null,
    refreshInterval: null,
    connectionType: 'rest',
    recordFromHour: null,
    recordToHour: null,
  });
}

/** 刪除單一訂閱 */
export async function removeSubscription(id: number): Promise<void> {
  await invoke('remove_subscription', { id });
}

/** 批量刪除訂閱 */
export async function removeSubscriptions(ids: number[]): Promise<void> {
  if (ids.length === 0) return;
  await invoke('remove_subscriptions', { ids });
}
