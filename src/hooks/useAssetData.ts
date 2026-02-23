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

  // key-based listeners: "providerId:symbol" → Set<callback>
  private keyListeners = new Map<string, Set<() => void>>();
  // provider-based listeners: "providerId" → Set<callback> (for tick updates)
  private tickListeners = new Map<string, Set<() => void>>();

  getAsset(key: string) { return this.assets.get(key); }
  getError(key: string) { return this.errors.get(key); }
  getTick(providerId: string) { return this.ticks.get(providerId); }

  /** 批量更新價格 — 只通知有變化的 key */
  updatePrices(results: AssetData[]) {
    for (const d of results) {
      const key = `${d.provider_id}:${d.symbol}`;
      const prev = this.assets.get(key);
      let changed = false;
      if (!prev || prev.price !== d.price || prev.last_updated !== d.last_updated) {
        this.assets.set(key, d);
        changed = true;
      }
      if (this.errors.has(key)) {
        this.errors.delete(key);
        changed = true;
      }
      if (changed) this.notifyKey(key);
    }
  }

  updateErrors(payload: Record<string, string>) {
    for (const [k, msg] of Object.entries(payload)) {
      if (this.errors.get(k) !== msg) {
        this.errors.set(k, msg);
        this.notifyKey(k);
      }
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

  clear() {
    this.assets.clear();
    this.errors.clear();
    this.ticks.clear();
  }

  /** 訂閱特定 asset key 的變化 */
  subscribeKey(key: string, fn: () => void) {
    let set = this.keyListeners.get(key);
    if (!set) { set = new Set(); this.keyListeners.set(key, set); }
    set.add(fn);
    return () => {
      set!.delete(fn);
      if (set!.size === 0) this.keyListeners.delete(key);
    };
  }

  /** 訂閱特定 provider 的 tick 變化 */
  subscribeTick(providerId: string, fn: () => void) {
    let set = this.tickListeners.get(providerId);
    if (!set) { set = new Set(); this.tickListeners.set(providerId, set); }
    set.add(fn);
    return () => {
      set!.delete(fn);
      if (set!.size === 0) this.tickListeners.delete(providerId);
    };
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

/**
 * 細粒度 hook — 只在自己的 key 變化時 re-render（O(1) 通知）
 */
export function useAssetPrice(symbol: string, providerId: string) {
  const key = `${providerId}:${symbol}`;
  const [, setTick] = useState(0);

  useEffect(() => {
    return priceStore.subscribeKey(key, () => setTick(t => t + 1));
  }, [key]);

  return {
    asset: priceStore.getAsset(key),
    error: priceStore.getError(key),
  };
}

/**
 * 取得後端 poll-tick — 只在對應 provider 的 tick 變化時 re-render
 */
export function usePollTick(providerId: string) {
  const [tick, setTick] = useState(() => priceStore.getTick(providerId));

  useEffect(() => {
    return priceStore.subscribeTick(providerId, () => {
      setTick(priceStore.getTick(providerId));
    });
  }, [providerId]);

  return tick;
}

export function useAssetData() {
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [providerInfoList, setProviderInfoList] = useState<ProviderInfo[]>([]);
  const [loading, setLoading] = useState(true);

  const wsUnlistenRef = useRef<UnlistenFn | null>(null);
  const wsActiveRef = useRef<Set<string>>(new Set());
  const priceUnlistenRef = useRef<UnlistenFn | null>(null);
  const errorUnlistenRef = useRef<UnlistenFn | null>(null);
  const providerInfoRef = useRef<ProviderInfo[]>([]);
  const subscriptionsRef = useRef<Subscription[]>([]);
  subscriptionsRef.current = subscriptions;

  const selectedProviders = useMemo(() => {
    const map = new Map<number, string>();
    for (const sub of subscriptions) map.set(sub.id, sub.selected_provider_id);
    return map;
  }, [subscriptions]);

  // ── Data Loading ────────────────────────────────────────────

  const loadProviderInfo = useCallback(async () => {
    try {
      const info = await invoke<ProviderInfo[]>('get_all_providers');
      setProviderInfoList(info);
      providerInfoRef.current = info;
    } catch (err) {
      console.error('Failed to load provider info:', err);
    }
  }, []);

  const loadSubscriptions = useCallback(async () => {
    try {
      const db = await getDb();
      const result = await db.select<Subscription[]>(
        'SELECT id, symbol, display_name, selected_provider_id, asset_type, sort_order FROM subscriptions ORDER BY sort_order, id'
      );
      setSubscriptions(result);
      return result;
    } catch (err) {
      console.error('Failed to load subscriptions:', err);
      return [];
    }
  }, []);

  const loadCachedPrices = useCallback(async () => {
    try {
      const cached = await invoke<AssetData[]>('get_cached_prices');
      if (cached.length > 0) priceStore.updatePrices(cached);
    } catch (err) {
      console.error('Failed to load cached prices:', err);
    }
  }, []);

  const loadCachedTicks = useCallback(async () => {
    try {
      const ticks = await invoke<{ provider_id: string; fetched_at: number; interval_ms: number }[]>('get_poll_ticks');
      for (const t of ticks) {
        priceStore.updateTick(t.provider_id, t.fetched_at, t.interval_ms);
      }
    } catch (err) {
      console.error('Failed to load cached ticks:', err);
    }
  }, []);

  // ── Event Listeners ─────────────────────────────────────────

  const setupPriceListener = useCallback(async () => {
    if (priceUnlistenRef.current) return;
    priceUnlistenRef.current = await listen<AssetData[]>('price-update', (event) => {
      priceStore.updatePrices(event.payload);
    });
  }, []);

  const setupErrorListener = useCallback(async () => {
    if (errorUnlistenRef.current) return;
    errorUnlistenRef.current = await listen<Record<string, string>>('price-error', (event) => {
      priceStore.updateErrors(event.payload);
    });
  }, []);

  const tickUnlistenRef = useRef<UnlistenFn | null>(null);
  const setupTickListener = useCallback(async () => {
    if (tickUnlistenRef.current) return;
    tickUnlistenRef.current = await listen<{ provider_id: string; fetched_at: number; interval_ms: number }>('poll-tick', (event) => {
      const { provider_id, fetched_at, interval_ms } = event.payload;
      priceStore.updateTick(provider_id, fetched_at, interval_ms);
    });
  }, []);

  const setupWsListener = useCallback(async () => {
    if (wsUnlistenRef.current) return;
    wsUnlistenRef.current = await listen<WsTickerUpdate>('ws-ticker-update', (event) => {
      const { provider_id, symbol, data } = event.payload;
      priceStore.updateWs(provider_id, symbol, data);
    });
  }, []);

  // ── WebSocket ───────────────────────────────────────────────

  const startWsStream = useCallback(async (providerId: string, symbols: string[]) => {
    const key = `${providerId}:${symbols.join(',')}`;
    if (wsActiveRef.current.has(key)) return;
    try {
      await invoke('start_ws_stream', { providerId, symbols });
      wsActiveRef.current.add(key);
      await setupWsListener();
    } catch (err) {
      console.error(`WS stream failed for ${providerId}:`, err);
    }
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
      for (const [pid, syms] of Object.entries(groups)) {
        startWsStream(pid, syms);
      }
    } catch (err) {
      console.error('Failed to start WS connections:', err);
    }
  }, [startWsStream]);

  // ── Polling ─────────────────────────────────────────────────

  const reloadPolling = useCallback(async () => {
    try {
      await invoke('reload_polling');
    } catch (err) {
      console.error('Failed to reload polling:', err);
    }
  }, []);

  // ── CRUD Operations ─────────────────────────────────────────

  const addSubscription = useCallback(
    async (symbol: string, displayName?: string, providerId?: string, assetType?: 'crypto' | 'stock') => {
      const db = await getDb();
      // DEX 聚合器使用合約地址，需保留原始大小寫
      const isDexProvider = providerInfoRef.current.find(p => p.id === providerId)?.provider_type === 'dex';
      const storedSymbol = isDexProvider ? symbol.trim() : symbol.toUpperCase();
      await db.execute(
        'INSERT INTO subscriptions (symbol, display_name, selected_provider_id, asset_type) VALUES ($1, $2, $3, $4)',
        [storedSymbol, displayName || null, providerId || 'binance', assetType || 'crypto']
      );
      await loadSubscriptions();
    },
    [loadSubscriptions]
  );

  const updateSubscription = useCallback(
    async (id: number, updates: { symbol?: string; displayName?: string; providerId?: string; assetType?: 'crypto' | 'stock' }) => {
      const sub = subscriptionsRef.current.find(s => s.id === id);
      if (!sub) return;
      const db = await getDb();
      const targetProviderId = updates.providerId ?? sub.selected_provider_id;
      const isDexProvider = providerInfoRef.current.find(p => p.id === targetProviderId)?.provider_type === 'dex';
      const storedSymbol = updates.symbol
        ? (isDexProvider ? updates.symbol.trim() : updates.symbol.toUpperCase())
        : sub.symbol;
      await db.execute(
        'UPDATE subscriptions SET symbol = $1, display_name = $2, selected_provider_id = $3, asset_type = $4 WHERE id = $5',
        [
          storedSymbol,
          updates.displayName !== undefined ? (updates.displayName || null) : (sub.display_name || null),
          targetProviderId,
          updates.assetType ?? sub.asset_type,
          id,
        ]
      );
      await loadSubscriptions();
      // symbol 或 provider 變更時 visible IDs 不變，set_visible_subscriptions 不會觸發，
      // 需要手動 reload 讓 backend 重新讀取 DB 配置
      if (updates.symbol || updates.providerId) {
        await reloadPolling();
      }
    },
    [loadSubscriptions, reloadPolling]
  );

  const removeSubscription = useCallback(
    async (id: number) => {
      const db = await getDb();
      await db.execute('DELETE FROM subscriptions WHERE id = $1', [id]);
      await loadSubscriptions();
    },
    [loadSubscriptions]
  );

  const removeSubscriptions = useCallback(
    async (ids: number[]) => {
      if (ids.length === 0) return;
      const db = await getDb();
      const placeholders = ids.map((_, i) => '$' + (i + 1)).join(',');
      await db.execute(`DELETE FROM subscriptions WHERE id IN (${placeholders})`, ids);
      await loadSubscriptions();
    },
    [loadSubscriptions]
  );

  const reorderSubscriptions = useCallback(
    async (orderedIds: number[]) => {
      const db = await getDb();
      for (let i = 0; i < orderedIds.length; i++) {
        await db.execute('UPDATE subscriptions SET sort_order = $1 WHERE id = $2', [i + 1, orderedIds[i]]);
      }
      await loadSubscriptions();
    },
    [loadSubscriptions]
  );

  // ── Getters (穩定引用，不觸發 re-render) ───────────────────

  const getSelectedProvider = useCallback(
    (subscriptionId: number): string => selectedProviders.get(subscriptionId) || 'binance',
    [selectedProviders]
  );

  const getAssetType = useCallback(
    (subscriptionId: number): 'crypto' | 'stock' => {
      return subscriptionsRef.current.find(s => s.id === subscriptionId)?.asset_type || 'crypto';
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
      await startWsConnections(subs);
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
    removeSubscription,
    removeSubscriptions,
    updateSubscription,
    reorderSubscriptions,
    getSelectedProvider,
    getAssetType,
    getRefreshInterval,
    refresh: reloadPolling,
  };
}
