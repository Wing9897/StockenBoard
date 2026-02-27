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

  const unlistenRefs = useRef<UnlistenFn[]>([]);
  const wsActiveRef = useRef<Set<string>>(new Set());
  const providerInfoRef = useRef<ProviderInfo[]>([]);
  const subscriptionsRef = useRef<Subscription[]>([]);
  subscriptionsRef.current = subscriptions;

  const selectedProviders = useMemo(() => {
    const map = new Map<number, string>();
    for (const sub of subscriptions) map.set(sub.id, sub.selected_provider_id);
    return map;
  }, [subscriptions]);

  const loadSubs = useCallback(async () => {
    try {
      const result = await api.loadSubscriptions(subType);
      setSubscriptions(result);
      return result;
    } catch { return []; }
  }, [subType]);

  // ── Init ──
  useEffect(() => {
    (async () => {
      // 載入 provider info
      try {
        const info = await api.loadProviderInfo();
        setProviderInfoList(info);
        providerInfoRef.current = info;
      } catch { /* silent */ }

      const subs = await loadSubs();

      // 設定所有 event listeners
      const fns = await Promise.all([
        listen<AssetData[]>('price-update', e => priceStore.updatePrices(e.payload)),
        listen<Record<string, string>>('price-error', e => priceStore.updateErrors(e.payload)),
        listen<{ provider_id: string; fetched_at: number; interval_ms: number }>('poll-tick', e => {
          priceStore.updateTick(e.payload.provider_id, e.payload.fetched_at, e.payload.interval_ms);
        }),
        listen<WsTickerUpdate>('ws-ticker-update', e => {
          priceStore.updateWs(e.payload.provider_id, e.payload.symbol, e.payload.data);
        }),
      ]);
      unlistenRefs.current = fns;

      // 載入快取
      try {
        const cached = await invoke<AssetData[]>('get_cached_prices');
        if (cached.length > 0) priceStore.updatePrices(cached);
      } catch { /* silent */ }
      try {
        const ticks = await invoke<{ provider_id: string; fetched_at: number; interval_ms: number }[]>('get_poll_ticks');
        for (const t of ticks) priceStore.updateTick(t.provider_id, t.fetched_at, t.interval_ms);
      } catch { /* silent */ }

      // WebSocket 連線（僅 asset）
      if (subType === 'asset') {
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
          for (const [pid, syms] of Object.entries(groups)) {
            const key = `${pid}:${syms.join(',')}`;
            if (!wsActiveRef.current.has(key)) {
              await invoke('start_ws_stream', { providerId: pid, symbols: syms });
              wsActiveRef.current.add(key);
            }
          }
        } catch { /* silent */ }
      }

      setLoading(false);
    })();
    return () => { for (const fn of unlistenRefs.current) fn(); };
  }, []);

  // ── CRUD ──

  const addSubscription = useCallback(
    async (symbol: string, displayName?: string, providerId?: string, assetType?: string) => {
      await api.addAssetSubscription(symbol, providerInfoRef.current, displayName, providerId, assetType);
      await loadSubs();
      await api.reloadPolling();
    }, [loadSubs]);

  const addSubscriptionBatch = useCallback(
    async (items: { symbol: string; displayName?: string; providerId?: string; assetType?: string }[], onProgress?: (done: number, total: number) => void) => {
      const result = await api.addAssetSubscriptionBatch(items, providerInfoRef.current, onProgress);
      if (result.succeeded.length > 0) { await loadSubs(); await api.reloadPolling(); }
      return result;
    }, [loadSubs]);

  const addDexSubscription = useCallback(
    async (poolAddress: string, tokenFrom: string, tokenTo: string, providerId: string, displayName?: string) => {
      await api.addDexSubscription(poolAddress, tokenFrom, tokenTo, providerId, displayName);
      await loadSubs();
      await api.reloadPolling();
    }, [loadSubs]);

  const updateSubscription = useCallback(
    async (id: number, updates: { symbol?: string; displayName?: string; providerId?: string; assetType?: string }) => {
      const sub = subscriptionsRef.current.find(s => s.id === id);
      if (!sub) return;
      const needsReload = await api.updateAssetSubscription(sub, providerInfoRef.current, updates);
      await loadSubs();
      if (needsReload) await api.reloadPolling();
    }, [loadSubs]);

  const updateDexSubscription = useCallback(
    async (id: number, updates: { poolAddress?: string; tokenFrom?: string; tokenTo?: string; providerId?: string; displayName?: string }) => {
      const sub = subscriptionsRef.current.find(s => s.id === id);
      if (!sub) return;
      await api.updateDexSub(sub, updates);
      await loadSubs();
      await api.reloadPolling();
    }, [loadSubs]);

  const removeSubscription = useCallback(async (id: number) => {
    await api.removeSubscription(id);
    await loadSubs();
  }, [loadSubs]);

  const removeSubscriptions = useCallback(async (ids: number[]) => {
    await api.removeSubscriptions(ids);
    await loadSubs();
    await api.reloadPolling();
  }, [loadSubs]);

  // ── Getters ──

  const getSelectedProvider = useCallback(
    (subscriptionId: number): string => selectedProviders.get(subscriptionId) || 'binance',
    [selectedProviders]);

  const getAssetType = useCallback(
    (subscriptionId: number): 'crypto' | 'stock' =>
      (subscriptionsRef.current.find(s => s.id === subscriptionId)?.asset_type as 'crypto' | 'stock') || 'crypto',
    []);

  const getRefreshInterval = useCallback((providerId: string): number => {
    const tick = priceStore.getTick(providerId);
    if (tick) return tick.intervalMs;
    return providerInfoRef.current.find(i => i.id === providerId)?.free_interval || 30000;
  }, []);

  const getDexSymbol = useCallback((sub: Subscription): string =>
    `${sub.pool_address || ''}:${sub.token_from_address || ''}:${sub.token_to_address || ''}`, []);

  return {
    subscriptions, providerInfoList, loading,
    addSubscription, addSubscriptionBatch, addDexSubscription,
    removeSubscription, removeSubscriptions,
    updateSubscription, updateDexSubscription,
    getSelectedProvider, getAssetType, getRefreshInterval, getDexSymbol,
    refresh: useCallback(async () => { await api.reloadPolling(); }, []),
  };
}
