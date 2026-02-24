import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { AssetData, Subscription, ProviderInfo, WsTickerUpdate } from '../types';
import { getDb } from '../lib/db';

/**
 * 高效價格 store — 使用 key-based subscription，
 * 每張卡片只在自己的 key 變化時收到通知，避免 O(N) 廣播。
 */
class PriceStore {
  private assets = new Map<string, AssetData>();
  private errors = new Map<string, string>();
  private ticks = new Map<string, { fetchedAt: number; intervalMs: number }>();
  private keyListeners = new Map<string, Set<() => void>>();
  private tickListeners = new Map<string, Set<() => void>>();

  getAsset(key: string) { return this.assets.get(key); }
  getError(key: string) { return this.errors.get(key); }
  getTick(providerId: string) { return this.ticks.get(providerId); }

  updatePrices(results: AssetData[]) {
    for (const d of results) {
      const key = `${d.provider_id}:${d.symbol}`;
      const prev = this.assets.get(key);
      let changed = false;
      if (!prev || prev.price !== d.price || prev.last_updated !== d.last_updated) {
        this.assets.set(key, d);
        changed = true;
      }
      if (this.errors.has(key)) { this.errors.delete(key); changed = true; }
      if (changed) this.notifyKey(key);
    }
  }

  updateErrors(payload: Record<string, string>) {
    for (const [k, msg] of Object.entries(payload)) {
      if (this.errors.get(k) !== msg) { this.errors.set(k, msg); this.notifyKey(k); }
    }
  }

  updateTick(providerId: string, fetchedAt: number, intervalMs: number) {
    const prev = this.ticks.get(providerId);
    if (prev && prev.fetchedAt === fetchedAt && prev.intervalMs === intervalMs) return;
    this.ticks.set(providerId, { fetchedAt, intervalMs });
    this.notifyTick(providerId);
  }

  updateWs(providerId: string, symbol: string, data: AssetData) {
    const key = `${providerId}:${symbol}`;
    this.assets.set(key, data);
    this.notifyKey(key);
  }

  clear() { this.assets.clear(); this.errors.clear(); this.ticks.clear(); }

  subscribeKey(key: string, fn: () => void) {
    let set = this.keyListeners.get(key);
    if (!set) { set = new Set(); this.keyListeners.set(key, set); }
    set.add(fn);
    return () => { set!.delete(fn); if (set!.size === 0) this.keyListeners.delete(key); };
  }

  subscribeTick(providerId: string, fn: () => void) {
    let set = this.tickListeners.get(providerId);
    if (!set) { set = new Set(); this.tickListeners.set(providerId, set); }
    set.add(fn);
    return () => { set!.delete(fn); if (set!.size === 0) this.tickListeners.delete(providerId); };
  }

  private notifyKey(key: string) {
    const fns = this.keyListeners.get(key);
    if (fns) for (const fn of fns) fn();
  }
  private notifyTick(providerId: string) {
    const fns = this.tickListeners.get(providerId);
    if (fns) for (const fn of fns) fn();
  }
}

const priceStore = new PriceStore();

export function useAssetPrice(symbol: string, providerId: string) {
  const key = `${providerId}:${symbol}`;
  const [, setTick] = useState(0);
  useEffect(() => priceStore.subscribeKey(key, () => setTick(t => t + 1)), [key]);
  return { asset: priceStore.getAsset(key), error: priceStore.getError(key) };
}

export function usePollTick(providerId: string) {
  const [tick, setTick] = useState(() => priceStore.getTick(providerId));
  useEffect(() => priceStore.subscribeTick(providerId, () => setTick(priceStore.getTick(providerId))), [providerId]);
  return tick;
}

/** 統一 hook — 透過 subType 參數區分 asset / dex */
export function useAssetData(subType: 'asset' | 'dex' = 'asset') {
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [providerInfoList, setProviderInfoList] = useState<ProviderInfo[]>([]);
  const [loading, setLoading] = useState(true);

  const wsUnlistenRef = useRef<UnlistenFn | null>(null);
  const wsActiveRef = useRef<Set<string>>(new Set());
  const priceUnlistenRef = useRef<UnlistenFn | null>(null);
  const errorUnlistenRef = useRef<UnlistenFn | null>(null);
  const tickUnlistenRef = useRef<UnlistenFn | null>(null);
  const providerInfoRef = useRef<ProviderInfo[]>([]);
  const subscriptionsRef = useRef<Subscription[]>([]);
  subscriptionsRef.current = subscriptions;

  const selectedProviders = useMemo(() => {
    const map = new Map<number, string>();
    for (const sub of subscriptions) map.set(sub.id, sub.selected_provider_id);
    return map;
  }, [subscriptions]);

  const loadProviderInfo = useCallback(async () => {
    try {
      const info = await invoke<ProviderInfo[]>('get_all_providers');
      setProviderInfoList(info);
      providerInfoRef.current = info;
    } catch (err) { console.error('Failed to load provider info:', err); }
  }, []);

  const loadSubscriptions = useCallback(async () => {
    try {
      const db = await getDb();
      const result = await db.select<Subscription[]>(
        'SELECT id, sub_type, symbol, display_name, selected_provider_id, asset_type, pool_address, token_from_address, token_to_address, sort_order FROM subscriptions WHERE sub_type = $1 ORDER BY sort_order, id',
        [subType]
      );
      setSubscriptions(result);
      return result;
    } catch (err) { console.error('Failed to load subscriptions:', err); return []; }
  }, [subType]);

  const loadCachedPrices = useCallback(async () => {
    try {
      const cached = await invoke<AssetData[]>('get_cached_prices');
      if (cached.length > 0) priceStore.updatePrices(cached);
    } catch (err) { console.error('Failed to load cached prices:', err); }
  }, []);

  const loadCachedTicks = useCallback(async () => {
    try {
      const ticks = await invoke<{ provider_id: string; fetched_at: number; interval_ms: number }[]>('get_poll_ticks');
      for (const t of ticks) priceStore.updateTick(t.provider_id, t.fetched_at, t.interval_ms);
    } catch (err) { console.error('Failed to load cached ticks:', err); }
  }, []);

  // Event listeners
  const setupPriceListener = useCallback(async () => {
    if (priceUnlistenRef.current) return;
    priceUnlistenRef.current = await listen<AssetData[]>('price-update', (e) => priceStore.updatePrices(e.payload));
  }, []);
  const setupErrorListener = useCallback(async () => {
    if (errorUnlistenRef.current) return;
    errorUnlistenRef.current = await listen<Record<string, string>>('price-error', (e) => priceStore.updateErrors(e.payload));
  }, []);
  const setupTickListener = useCallback(async () => {
    if (tickUnlistenRef.current) return;
    tickUnlistenRef.current = await listen<{ provider_id: string; fetched_at: number; interval_ms: number }>('poll-tick', (e) => {
      const { provider_id, fetched_at, interval_ms } = e.payload;
      priceStore.updateTick(provider_id, fetched_at, interval_ms);
    });
  }, []);
  const setupWsListener = useCallback(async () => {
    if (wsUnlistenRef.current) return;
    wsUnlistenRef.current = await listen<WsTickerUpdate>('ws-ticker-update', (e) => {
      const { provider_id, symbol, data } = e.payload;
      priceStore.updateWs(provider_id, symbol, data);
    });
  }, []);

  // WebSocket
  const startWsStream = useCallback(async (providerId: string, symbols: string[]) => {
    const key = `${providerId}:${symbols.join(',')}`;
    if (wsActiveRef.current.has(key)) return;
    try {
      await invoke('start_ws_stream', { providerId, symbols });
      wsActiveRef.current.add(key);
      await setupWsListener();
    } catch (err) { console.error(`WS stream failed for ${providerId}:`, err); }
  }, [setupWsListener]);

  const startWsConnections = useCallback(async (subs: Subscription[]) => {
    try {
      const db = await getDb();
      const settings = await db.select<{ provider_id: string }[]>(
        "SELECT provider_id FROM provider_settings WHERE connection_type = 'websocket' AND enabled = 1"
      );
      const wsProviders = new Set(settings.map(s => s.provider_id));
      const groups: Record<string, string[]> = {};
      for (const sub of subs) {
        if (wsProviders.has(sub.selected_provider_id)) {
          (groups[sub.selected_provider_id] ??= []).push(sub.symbol);
        }
      }
      for (const [pid, syms] of Object.entries(groups)) startWsStream(pid, syms);
    } catch (err) { console.error('Failed to start WS connections:', err); }
  }, [startWsStream]);

  const reloadPolling = useCallback(async () => {
    try { await invoke('reload_polling'); } catch (err) { console.error('Failed to reload polling:', err); }
  }, []);

  // ── CRUD ────────────────────────────────────────────────────

  /** 新增 asset 訂閱 */
  const addSubscription = useCallback(
    async (symbol: string, displayName?: string, providerId?: string, assetType?: string) => {
      const pid = providerId || 'binance';
      const isDex = providerInfoRef.current.find(p => p.id === pid)?.provider_type === 'dex';
      const storedSymbol = isDex ? symbol.trim() : symbol.toUpperCase();
      // 先驗證 symbol 是否有效
      await invoke('fetch_asset_price', { providerId: pid, symbol: storedSymbol });
      const db = await getDb();
      await db.execute(
        'INSERT INTO subscriptions (sub_type, symbol, display_name, selected_provider_id, asset_type) VALUES ($1, $2, $3, $4, $5)',
        ['asset', storedSymbol, displayName || null, pid, assetType || 'crypto']
      );
      await loadSubscriptions();
      await reloadPolling();
    },
    [loadSubscriptions, reloadPolling]
  );

  /** 新增 DEX 訂閱 */
  const addDexSubscription = useCallback(
    async (poolAddress: string, tokenFrom: string, tokenTo: string, providerId: string, displayName?: string) => {
      const db = await getDb();
      const symbol = `${poolAddress.trim()}:${tokenFrom.trim()}:${tokenTo.trim()}`;
      await db.execute(
        'INSERT INTO subscriptions (sub_type, symbol, display_name, selected_provider_id, asset_type, pool_address, token_from_address, token_to_address) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)',
        ['dex', symbol, displayName || null, providerId, 'crypto', poolAddress.trim(), tokenFrom.trim(), tokenTo.trim()]
      );
      await loadSubscriptions();
      await reloadPolling();
    },
    [loadSubscriptions, reloadPolling]
  );

  const updateSubscription = useCallback(
    async (id: number, updates: { symbol?: string; displayName?: string; providerId?: string; assetType?: string }) => {
      const sub = subscriptionsRef.current.find(s => s.id === id);
      if (!sub) return;
      const db = await getDb();
      const targetPid = updates.providerId ?? sub.selected_provider_id;
      const isDex = providerInfoRef.current.find(p => p.id === targetPid)?.provider_type === 'dex';
      const storedSymbol = updates.symbol ? (isDex ? updates.symbol.trim() : updates.symbol.toUpperCase()) : sub.symbol;
      await db.execute(
        'UPDATE subscriptions SET symbol = $1, display_name = $2, selected_provider_id = $3, asset_type = $4 WHERE id = $5',
        [storedSymbol, updates.displayName !== undefined ? (updates.displayName || null) : (sub.display_name || null), targetPid, updates.assetType ?? sub.asset_type, id]
      );
      await loadSubscriptions();
      if (updates.symbol || updates.providerId) await reloadPolling();
    },
    [loadSubscriptions, reloadPolling]
  );

  /** 更新 DEX 訂閱 */
  const updateDexSubscription = useCallback(
    async (id: number, updates: { poolAddress?: string; tokenFrom?: string; tokenTo?: string; providerId?: string; displayName?: string }) => {
      const sub = subscriptionsRef.current.find(s => s.id === id);
      if (!sub) return;
      const db = await getDb();
      const pool = updates.poolAddress?.trim() ?? sub.pool_address ?? '';
      const tf = updates.tokenFrom?.trim() ?? sub.token_from_address ?? '';
      const tt = updates.tokenTo?.trim() ?? sub.token_to_address ?? '';
      const symbol = `${pool}:${tf}:${tt}`;
      await db.execute(
        'UPDATE subscriptions SET symbol = $1, pool_address = $2, token_from_address = $3, token_to_address = $4, selected_provider_id = $5, display_name = $6 WHERE id = $7',
        [symbol, pool, tf, tt, updates.providerId ?? sub.selected_provider_id, updates.displayName !== undefined ? (updates.displayName || null) : (sub.display_name || null), id]
      );
      await loadSubscriptions();
      await reloadPolling();
    },
    [loadSubscriptions, reloadPolling]
  );

  const removeSubscription = useCallback(async (id: number) => {
    const db = await getDb();
    await db.execute('DELETE FROM subscriptions WHERE id = $1', [id]);
    await loadSubscriptions();
  }, [loadSubscriptions]);

  const removeSubscriptions = useCallback(async (ids: number[]) => {
    if (ids.length === 0) return;
    const db = await getDb();
    const placeholders = ids.map((_, i) => '$' + (i + 1)).join(',');
    await db.execute(`DELETE FROM subscriptions WHERE id IN (${placeholders})`, ids);
    await loadSubscriptions();
  }, [loadSubscriptions]);

  const reorderSubscriptions = useCallback(async (orderedIds: number[]) => {
    const db = await getDb();
    for (let i = 0; i < orderedIds.length; i++) {
      await db.execute('UPDATE subscriptions SET sort_order = $1 WHERE id = $2', [i + 1, orderedIds[i]]);
    }
    await loadSubscriptions();
  }, [loadSubscriptions]);

  // ── Getters ─────────────────────────────────────────────────

  const getSelectedProvider = useCallback(
    (subscriptionId: number): string => selectedProviders.get(subscriptionId) || 'binance',
    [selectedProviders]
  );

  const getAssetType = useCallback(
    (subscriptionId: number): 'crypto' | 'stock' => {
      return (subscriptionsRef.current.find(s => s.id === subscriptionId)?.asset_type as 'crypto' | 'stock') || 'crypto';
    },
    []
  );

  const getRefreshInterval = useCallback(
    (providerId: string): number => {
      const tick = priceStore.getTick(providerId);
      if (tick) return tick.intervalMs;
      const info = providerInfoRef.current.find(i => i.id === providerId);
      return info?.free_interval || 30000;
    },
    []
  );

  /** DEX symbol 組合 */
  const getDexSymbol = useCallback((sub: Subscription): string => {
    return `${sub.pool_address || ''}:${sub.token_from_address || ''}:${sub.token_to_address || ''}`;
  }, []);

  // ── Init ────────────────────────────────────────────────────

  useEffect(() => {
    (async () => {
      await loadProviderInfo();
      const subs = await loadSubscriptions();
      await setupPriceListener();
      await setupErrorListener();
      await setupTickListener();
      await loadCachedPrices();
      await loadCachedTicks();
      if (subType === 'asset') await startWsConnections(subs);
      setLoading(false);
    })();
    return () => {
      wsUnlistenRef.current?.();
      priceUnlistenRef.current?.();
      errorUnlistenRef.current?.();
      tickUnlistenRef.current?.();
    };
  }, []);

  return {
    subscriptions,
    providerInfoList,
    loading,
    addSubscription,
    addDexSubscription,
    removeSubscription,
    removeSubscriptions,
    updateSubscription,
    updateDexSubscription,
    reorderSubscriptions,
    getSelectedProvider,
    getAssetType,
    getRefreshInterval,
    getDexSymbol,
    refresh: reloadPolling,
  };
}
