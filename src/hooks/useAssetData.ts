import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import Database from '@tauri-apps/plugin-sql';
import { AssetData, Subscription, Provider, ProviderInfo, WsTickerUpdate } from '../types';

export function useAssetData() {
  const [assets, setAssets] = useState<Map<string, AssetData>>(new Map());
  const [errors, setErrors] = useState<Map<string, string>>(new Map());
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [selectedProviders, setSelectedProviders] = useState<Map<number, string>>(new Map());
  const [providerInfoList, setProviderInfoList] = useState<ProviderInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const intervalsRef = useRef<Map<string, NodeJS.Timeout>>(new Map());
  const refreshTimingRef = useRef<Map<number, { interval: number; lastFetch: number }>>(new Map());
  const [refreshTimings, setRefreshTimings] = useState<Map<number, { interval: number; lastFetch: number }>>(new Map());
  const wsUnlistenRef = useRef<UnlistenFn | null>(null);
  const wsActiveRef = useRef<Set<string>>(new Set());

  const loadProviderInfo = useCallback(async () => {
    try {
      const info = await invoke<ProviderInfo[]>('get_all_providers');
      setProviderInfoList(info);
      return info;
    } catch (err) {
      console.error('Failed to load provider info:', err);
      return [];
    }
  }, []);

  const loadSubscriptions = useCallback(async () => {
    try {
      const db = await Database.load('sqlite:stockenboard.db');
      const result = await db.select<Subscription[]>(
        'SELECT id, symbol, display_name, icon_path, default_provider_id, selected_provider_id, asset_type, sort_order, created_at FROM subscriptions ORDER BY sort_order'
      );
      setSubscriptions(result);

      const newSelected = new Map<number, string>();
      result.forEach((sub) => {
        newSelected.set(sub.id, sub.selected_provider_id || sub.default_provider_id || 'binance');
      });
      setSelectedProviders(newSelected);
      return result;
    } catch (err) {
      console.error('Failed to load subscriptions:', err);
      return [];
    }
  }, []);

  // Setup WebSocket listener for real-time updates
  const setupWsListener = useCallback(async () => {
    if (wsUnlistenRef.current) return;
    const unlisten = await listen<WsTickerUpdate>('ws-ticker-update', (event) => {
      const update = event.payload;
      setAssets((prev) => {
        const next = new Map(prev);
        next.set(`${update.provider_id}:${update.symbol}`, update.data);
        return next;
      });
    });
    wsUnlistenRef.current = unlisten;
  }, []);

  // Start WebSocket stream for a provider+symbols
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

  // 批量取得價格：按 provider 將 symbol 合併為一次請求
  const fetchBatchPrices = useCallback(async (
    providerId: string,
    symbols: string[],
    subIdMap: Map<string, number>
  ) => {
    try {
      const results = await invoke<AssetData[]>('fetch_multiple_prices', { providerId, symbols });
      const now = Date.now();
      setAssets((prev) => {
        const next = new Map(prev);
        for (const data of results) {
          next.set(`${providerId}:${data.symbol}`, data);
        }
        return next;
      });
      setErrors((prev) => {
        const next = new Map(prev);
        for (const data of results) {
          next.delete(`${providerId}:${data.symbol}`);
        }
        return next;
      });
      for (const sym of symbols) {
        const subId = subIdMap.get(sym);
        if (subId !== undefined) {
          const existing = refreshTimingRef.current.get(subId);
          if (existing) { existing.lastFetch = now; }
        }
      }
      setRefreshTimings(new Map(refreshTimingRef.current));
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error(`Batch fetch failed for ${providerId}:`, msg);
      setErrors((prev) => {
        const next = new Map(prev);
        for (const sym of symbols) { next.set(`${providerId}:${sym}`, msg); }
        return next;
      });
      const now = Date.now();
      for (const sym of symbols) {
        const subId = subIdMap.get(sym);
        if (subId !== undefined) {
          const existing = refreshTimingRef.current.get(subId);
          if (existing) { existing.lastFetch = now; }
        }
      }
      setRefreshTimings(new Map(refreshTimingRef.current));
    }
  }, []);

  const startPolling = useCallback(async () => {
    intervalsRef.current.forEach((interval) => clearInterval(interval));
    intervalsRef.current.clear();

    const db = await Database.load('sqlite:stockenboard.db');
    const dbProviders = await db.select<Provider[]>('SELECT * FROM providers');
    const subs = await loadSubscriptions();

    const wsGroups: Record<string, string[]> = {};
    const restGroups: Record<string, { symbols: string[]; subIdMap: Map<string, number>; interval: number }> = {};

    subs.forEach((sub) => {
      const providerId = sub.selected_provider_id || sub.default_provider_id || 'binance';
      const provider = dbProviders.find((p) => p.id === providerId);
      const connectionType = provider?.connection_type || 'rest';

      if (connectionType === 'websocket') {
        if (!wsGroups[providerId]) wsGroups[providerId] = [];
        wsGroups[providerId].push(sub.symbol);
        refreshTimingRef.current.set(sub.id, { interval: 0, lastFetch: Date.now() });
      } else {
        const refreshInterval = provider?.refresh_interval || 30000;
        if (!restGroups[providerId]) {
          restGroups[providerId] = { symbols: [], subIdMap: new Map(), interval: refreshInterval };
        }
        restGroups[providerId].symbols.push(sub.symbol);
        restGroups[providerId].subIdMap.set(sub.symbol, sub.id);
        refreshTimingRef.current.set(sub.id, { interval: refreshInterval, lastFetch: Date.now() });
      }
    });

    for (const [providerId, group] of Object.entries(restGroups)) {
      const { symbols, subIdMap, interval } = group;
      fetchBatchPrices(providerId, symbols, subIdMap);
      const intervalId = setInterval(() => {
        fetchBatchPrices(providerId, symbols, subIdMap);
      }, interval);
      intervalsRef.current.set(`batch:${providerId}`, intervalId);
    }

    for (const [providerId, symbols] of Object.entries(wsGroups)) {
      const subIdMap = new Map<string, number>();
      subs.forEach(sub => {
        if (symbols.includes(sub.symbol)) subIdMap.set(sub.symbol, sub.id);
      });
      fetchBatchPrices(providerId, symbols, subIdMap);
      startWsStream(providerId, symbols);
    }

    setRefreshTimings(new Map(refreshTimingRef.current));
    setLoading(false);
  }, [fetchBatchPrices, loadSubscriptions, startWsStream]);

  const addSubscription = useCallback(
    async (symbol: string, displayName?: string, defaultProviderId?: string, assetType?: 'crypto' | 'stock') => {
      const db = await Database.load('sqlite:stockenboard.db');
      const providerId = defaultProviderId || 'binance';
      const type_ = assetType || 'crypto';
      await db.execute(
        'INSERT INTO subscriptions (symbol, display_name, default_provider_id, selected_provider_id, asset_type, sort_order) VALUES ($1, $2, $3, $4, $5, $6)',
        [symbol.toUpperCase(), displayName || null, providerId, providerId, type_, subscriptions.length]
      );
      await startPolling();
    },
    [subscriptions.length, startPolling]
  );

  const updateSubscription = useCallback(
    async (id: number, updates: { symbol?: string; displayName?: string; providerId?: string; assetType?: 'crypto' | 'stock' }) => {
      const db = await Database.load('sqlite:stockenboard.db');
      const sub = subscriptions.find(s => s.id === id);
      if (!sub) return;
      const newSymbol = updates.symbol?.toUpperCase() ?? sub.symbol;
      const newDisplayName = updates.displayName !== undefined ? (updates.displayName || null) : (sub.display_name || null);
      const newProvider = updates.providerId ?? sub.selected_provider_id ?? sub.default_provider_id ?? 'binance';
      const newAssetType = updates.assetType ?? sub.asset_type ?? 'crypto';
      await db.execute(
        'UPDATE subscriptions SET symbol = $1, display_name = $2, selected_provider_id = $3, asset_type = $4 WHERE id = $5',
        [newSymbol, newDisplayName, newProvider, newAssetType, id]
      );
      await startPolling();
    },
    [subscriptions, startPolling]
  );

  const removeSubscription = useCallback(
    async (id: number) => {
      const db = await Database.load('sqlite:stockenboard.db');
      await db.execute('DELETE FROM subscriptions WHERE id = $1', [id]);
      await startPolling();
    },
    [startPolling]
  );

  const getAsset = useCallback(
    (subscriptionId: number, symbol: string): AssetData | undefined => {
      const providerId = selectedProviders.get(subscriptionId) || 'binance';
      return assets.get(`${providerId}:${symbol}`);
    },
    [assets, selectedProviders]
  );

  const getError = useCallback(
    (subscriptionId: number, symbol: string): string | undefined => {
      const providerId = selectedProviders.get(subscriptionId) || 'binance';
      return errors.get(`${providerId}:${symbol}`);
    },
    [errors, selectedProviders]
  );

  const getSelectedProvider = useCallback(
    (subscriptionId: number): string => selectedProviders.get(subscriptionId) || 'binance',
    [selectedProviders]
  );

  const getAssetType = useCallback(
    (subscriptionId: number): 'crypto' | 'stock' => {
      const sub = subscriptions.find((s) => s.id === subscriptionId);
      return sub?.asset_type || 'crypto';
    },
    [subscriptions]
  );

  const getRefreshTiming = useCallback(
    (subscriptionId: number): { interval: number; lastFetch: number } | undefined => {
      return refreshTimings.get(subscriptionId);
    },
    [refreshTimings]
  );

  useEffect(() => {
    const init = async () => {
      loadProviderInfo();
      startPolling();
    };
    init();
    return () => {
      intervalsRef.current.forEach((interval) => clearInterval(interval));
      if (wsUnlistenRef.current) wsUnlistenRef.current();
    };
  }, []);

  return {
    assets, errors, subscriptions, providerInfoList, loading,
    addSubscription, removeSubscription, updateSubscription,
    getAsset, getError, getSelectedProvider, getAssetType, getRefreshTiming,
    refresh: startPolling,
  };
}
