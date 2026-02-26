import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import type { AssetData, Subscription, ProviderInfo, WsTickerUpdate } from '../types';
import { priceStore } from '../lib/priceStore';
import * as api from '../lib/subscriptionApi';
import { getDb } from '../lib/db';

// ── React hooks for subscribing to PriceStore ──

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

// ── Main orchestration hook ──

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

  // ── Loaders ──

  const loadProviderInfo = useCallback(async () => {
    try {
      const info = await api.loadProviderInfo();
      setProviderInfoList(info);
      providerInfoRef.current = info;
    } catch { /* silent */ }
  }, []);

  const loadSubscriptions = useCallback(async () => {
    try {
      const result = await api.loadSubscriptions(subType);
      setSubscriptions(result);
      return result;
    } catch { return []; }
  }, [subType]);

  const loadCachedPrices = useCallback(async () => {
    try {
      const cached = await invoke<AssetData[]>('get_cached_prices');
      if (cached.length > 0) priceStore.updatePrices(cached);
    } catch { /* silent */ }
  }, []);

  const loadCachedTicks = useCallback(async () => {
    try {
      const ticks = await invoke<{ provider_id: string; fetched_at: number; interval_ms: number }[]>('get_poll_ticks');
      for (const t of ticks) priceStore.updateTick(t.provider_id, t.fetched_at, t.interval_ms);
    } catch { /* silent */ }
  }, []);

  // ── Event listeners ──

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

  // ── WebSocket ──

  const startWsStream = useCallback(async (providerId: string, symbols: string[]) => {
    const key = `${providerId}:${symbols.join(',')}`;
    if (wsActiveRef.current.has(key)) return;
    try {
      await invoke('start_ws_stream', { providerId, symbols });
      wsActiveRef.current.add(key);
      await setupWsListener();
    } catch { /* silent */ }
  }, [setupWsListener]);

  const startWsConnections = useCallback(async (subs: Subscription[]) => {
    try {
      const db = await getDb();
      const settings = await db.select<{ provider_id: string }[]>(
        "SELECT provider_id FROM provider_settings WHERE connection_type = 'websocket'"
      );
      const wsProviders = new Set(settings.map(s => s.provider_id));
      const groups: Record<string, string[]> = {};
      for (const sub of subs) {
        if (wsProviders.has(sub.selected_provider_id)) {
          (groups[sub.selected_provider_id] ??= []).push(sub.symbol);
        }
      }
      for (const [pid, syms] of Object.entries(groups)) startWsStream(pid, syms);
    } catch { /* silent */ }
  }, [startWsStream]);

  // ── CRUD (thin wrappers delegating to subscriptionApi) ──

  const addSubscription = useCallback(
    async (symbol: string, displayName?: string, providerId?: string, assetType?: string) => {
      await api.addAssetSubscription(symbol, providerInfoRef.current, displayName, providerId, assetType);
      await loadSubscriptions();
      await api.reloadPolling();
    },
    [loadSubscriptions]
  );

  const addSubscriptionBatch = useCallback(
    async (
      items: { symbol: string; displayName?: string; providerId?: string; assetType?: string }[],
      onProgress?: (done: number, total: number) => void,
    ) => {
      const result = await api.addAssetSubscriptionBatch(items, providerInfoRef.current, onProgress);
      if (result.succeeded.length > 0) {
        await loadSubscriptions();
        await api.reloadPolling();
      }
      return result;
    },
    [loadSubscriptions]
  );

  const addDexSubscription = useCallback(
    async (poolAddress: string, tokenFrom: string, tokenTo: string, providerId: string, displayName?: string) => {
      await api.addDexSubscription(poolAddress, tokenFrom, tokenTo, providerId, displayName);
      await loadSubscriptions();
      await api.reloadPolling();
    },
    [loadSubscriptions]
  );

  const updateSubscription = useCallback(
    async (id: number, updates: { symbol?: string; displayName?: string; providerId?: string; assetType?: string }) => {
      const sub = subscriptionsRef.current.find(s => s.id === id);
      if (!sub) return;
      const needsReload = await api.updateAssetSubscription(sub, providerInfoRef.current, updates);
      await loadSubscriptions();
      if (needsReload) await api.reloadPolling();
    },
    [loadSubscriptions]
  );

  const updateDexSubscription = useCallback(
    async (id: number, updates: { poolAddress?: string; tokenFrom?: string; tokenTo?: string; providerId?: string; displayName?: string }) => {
      const sub = subscriptionsRef.current.find(s => s.id === id);
      if (!sub) return;
      await api.updateDexSub(sub, updates);
      await loadSubscriptions();
      await api.reloadPolling();
    },
    [loadSubscriptions]
  );

  const removeSubscription = useCallback(async (id: number) => {
    await api.removeSubscription(id);
    await loadSubscriptions();
  }, [loadSubscriptions]);

  const removeSubscriptions = useCallback(async (ids: number[]) => {
    await api.removeSubscriptions(ids);
    await loadSubscriptions();
    await api.reloadPolling();
  }, [loadSubscriptions]);

  // ── Getters ──

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

  const getDexSymbol = useCallback((sub: Subscription): string => {
    return `${sub.pool_address || ''}:${sub.token_from_address || ''}:${sub.token_to_address || ''}`;
  }, []);

  // ── Init ──

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
    subscriptions, providerInfoList, loading,
    addSubscription, addSubscriptionBatch, addDexSubscription,
    removeSubscription, removeSubscriptions,
    updateSubscription, updateDexSubscription,
    getSelectedProvider, getAssetType, getRefreshInterval, getDexSymbol,
    refresh: useCallback(async () => { await api.reloadPolling(); }, []),
  };
}
