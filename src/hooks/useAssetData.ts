import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import Database from '@tauri-apps/plugin-sql';
import { AssetData, Subscription, ProviderSettings, ProviderInfo, WsTickerUpdate } from '../types';

export function useAssetData() {
  const [assets, setAssets] = useState<Map<string, AssetData>>(new Map());
  const [errors, setErrors] = useState<Map<string, string>>(new Map());
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [providerInfoList, setProviderInfoList] = useState<ProviderInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const intervalsRef = useRef<Map<string, NodeJS.Timeout>>(new Map());
  const refreshTimingRef = useRef<Map<number, { interval: number; lastFetch: number }>>(new Map());
  const wsUnlistenRef = useRef<UnlistenFn | null>(null);
  const wsActiveRef = useRef<Set<string>>(new Set());
  // 用於取消過期的 startPolling 調用（快速切換 view 時防止競態）
  const pollingGenRef = useRef(0);

  // 快取 providerInfoList，供 startPolling 使用（避免每次 polling 都 IPC）
  const providerInfoRef = useRef<ProviderInfo[]>([]);

  // #1 簡化：selectedProviders 改為 subscriptions 的衍生資料（useMemo）
  const selectedProviders = useMemo(() => {
    const map = new Map<number, string>();
    subscriptions.forEach(sub => map.set(sub.id, sub.selected_provider_id));
    return map;
  }, [subscriptions]);

  const loadProviderInfo = useCallback(async () => {
    try {
      const info = await invoke<ProviderInfo[]>('get_all_providers');
      setProviderInfoList(info);
      providerInfoRef.current = info;
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
        'SELECT id, symbol, display_name, selected_provider_id, asset_type FROM subscriptions ORDER BY id'
      );
      setSubscriptions(result);
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

  // #2 簡化：refreshTimings 不再用 state，只用 ref
  // CountdownCircle 有自己的全域 timer 驅動 re-render，不需要靠 parent re-render
  // 更新 timing 只需寫入 ref，不需要 setState

  // 批量取得價格：按 provider 將 symbol 合併為一次請求
  const fetchBatchPrices = useCallback(async (
    providerId: string,
    symbols: string[],
    subIdMap: Map<string, number>
  ) => {
    const now = Date.now();
    try {
      const results = await invoke<AssetData[]>('fetch_multiple_prices', { providerId, symbols });
      setAssets((prev) => {
        const next = new Map(prev);
        for (const data of results) {
          next.set(`${providerId}:${data.symbol}`, data);
        }
        return next;
      });
      setErrors((prev) => {
        let changed = false;
        for (const data of results) {
          if (prev.has(`${providerId}:${data.symbol}`)) { changed = true; break; }
        }
        if (!changed) return prev;
        const next = new Map(prev);
        for (const data of results) next.delete(`${providerId}:${data.symbol}`);
        return next;
      });
      for (const sym of symbols) {
        const subId = subIdMap.get(sym);
        if (subId !== undefined) {
          const existing = refreshTimingRef.current.get(subId);
          if (existing) { existing.lastFetch = now; }
        }
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error(`Batch fetch failed for ${providerId}:`, msg);
      setErrors((prev) => {
        const next = new Map(prev);
        for (const sym of symbols) { next.set(`${providerId}:${sym}`, msg); }
        return next;
      });
      for (const sym of symbols) {
        const subId = subIdMap.get(sym);
        if (subId !== undefined) {
          const existing = refreshTimingRef.current.get(subId);
          if (existing) { existing.lastFetch = now; }
        }
      }
    }
  }, []);

  // activeSubIds: 當前頁面的 subscription IDs，null = 全部（default view）
  // WebSocket 永遠訂閱全部（斷開重連更浪費），REST polling 只 fetch 當前頁面
  const startPolling = useCallback(async (activeSubIds?: number[] | null) => {
    const gen = ++pollingGenRef.current;

    intervalsRef.current.forEach((interval) => clearInterval(interval));
    intervalsRef.current.clear();

    const db = await Database.load('sqlite:stockenboard.db');
    if (gen !== pollingGenRef.current) return;

    const dbSettings = await db.select<ProviderSettings[]>(
      'SELECT provider_id, api_key, api_secret, refresh_interval, connection_type FROM provider_settings'
    );
    const settingsMap = new Map<string, ProviderSettings>();
    for (const s of dbSettings) settingsMap.set(s.provider_id, s);

    const infos = providerInfoRef.current.length > 0
      ? providerInfoRef.current
      : await invoke<ProviderInfo[]>('get_all_providers');
    if (gen !== pollingGenRef.current) return;

    const getProviderConfig = (providerId: string) => {
      const info = infos.find(i => i.id === providerId);
      const s = settingsMap.get(providerId);
      const hasKey = !!s?.api_key;
      return {
        refresh_interval: s?.refresh_interval ?? (info ? (hasKey ? info.key_interval : info.free_interval) : 30000),
        connection_type: s?.connection_type || 'rest',
      };
    };

    const allSubs = await loadSubscriptions();
    if (gen !== pollingGenRef.current) return;

    const restSubs = activeSubIds == null
      ? allSubs
      : allSubs.filter(sub => activeSubIds.includes(sub.id));

    const wsGroups: Record<string, string[]> = {};
    const restGroups: Record<string, { symbols: string[]; subIdMap: Map<string, number>; interval: number }> = {};

    allSubs.forEach((sub) => {
      const providerId = sub.selected_provider_id;
      const config = getProviderConfig(providerId);
      if (config.connection_type === 'websocket') {
        if (!wsGroups[providerId]) wsGroups[providerId] = [];
        wsGroups[providerId].push(sub.symbol);
        refreshTimingRef.current.set(sub.id, { interval: 0, lastFetch: Date.now() });
      }
    });

    restSubs.forEach((sub) => {
      const providerId = sub.selected_provider_id;
      const config = getProviderConfig(providerId);
      if (config.connection_type !== 'websocket') {
        if (!restGroups[providerId]) {
          restGroups[providerId] = { symbols: [], subIdMap: new Map(), interval: config.refresh_interval };
        }
        restGroups[providerId].symbols.push(sub.symbol);
        restGroups[providerId].subIdMap.set(sub.symbol, sub.id);
        refreshTimingRef.current.set(sub.id, { interval: config.refresh_interval, lastFetch: Date.now() });
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
      allSubs.forEach(sub => {
        if (symbols.includes(sub.symbol)) subIdMap.set(sub.symbol, sub.id);
      });
      fetchBatchPrices(providerId, symbols, subIdMap);
      startWsStream(providerId, symbols);
    }

    // 清理不再使用的 refreshTiming entries
    const keepSubIds = new Set<number>();
    allSubs.forEach(sub => {
      const config = getProviderConfig(sub.selected_provider_id);
      if (config.connection_type === 'websocket') keepSubIds.add(sub.id);
    });
    restSubs.forEach(sub => keepSubIds.add(sub.id));
    for (const key of refreshTimingRef.current.keys()) {
      if (!keepSubIds.has(key)) refreshTimingRef.current.delete(key);
    }

    setLoading(false);
  }, [fetchBatchPrices, loadSubscriptions, startWsStream]);

  const activeSubIdsRef = useRef<number[] | null>(null);
  const subscriptionsRef = useRef<Subscription[]>([]);
  subscriptionsRef.current = subscriptions;

  const addSubscription = useCallback(
    async (symbol: string, displayName?: string, providerId?: string, assetType?: 'crypto' | 'stock') => {
      const db = await Database.load('sqlite:stockenboard.db');
      const pid = providerId || 'binance';
      const type_ = assetType || 'crypto';
      await db.execute(
        'INSERT INTO subscriptions (symbol, display_name, selected_provider_id, asset_type) VALUES ($1, $2, $3, $4)',
        [symbol.toUpperCase(), displayName || null, pid, type_]
      );
      await startPolling(null);
    },
    [startPolling]
  );

  const updateSubscription = useCallback(
    async (id: number, updates: { symbol?: string; displayName?: string; providerId?: string; assetType?: 'crypto' | 'stock' }) => {
      const db = await Database.load('sqlite:stockenboard.db');
      const sub = subscriptionsRef.current.find(s => s.id === id);
      if (!sub) return;
      const newSymbol = updates.symbol?.toUpperCase() ?? sub.symbol;
      const newDisplayName = updates.displayName !== undefined ? (updates.displayName || null) : (sub.display_name || null);
      const newProvider = updates.providerId ?? sub.selected_provider_id;
      const newAssetType = updates.assetType ?? sub.asset_type;
      await db.execute(
        'UPDATE subscriptions SET symbol = $1, display_name = $2, selected_provider_id = $3, asset_type = $4 WHERE id = $5',
        [newSymbol, newDisplayName, newProvider, newAssetType, id]
      );
      await startPolling(activeSubIdsRef.current);
    },
    [startPolling]
  );

  const removeSubscription = useCallback(
    async (id: number) => {
      const db = await Database.load('sqlite:stockenboard.db');
      await db.execute('DELETE FROM subscriptions WHERE id = $1', [id]);
      await startPolling(activeSubIdsRef.current);
    },
    [startPolling]
  );

  const removeSubscriptions = useCallback(
    async (ids: number[]) => {
      if (ids.length === 0) return;
      const db = await Database.load('sqlite:stockenboard.db');
      const placeholders = ids.map((_, i) => `$${i + 1}`).join(',');
      await db.execute(`DELETE FROM subscriptions WHERE id IN (${placeholders})`, ids);
      await startPolling(activeSubIdsRef.current);
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
      const sub = subscriptionsRef.current.find((s) => s.id === subscriptionId);
      return sub?.asset_type || 'crypto';
    },
    []
  );

  // #2 簡化：getRefreshTiming 直接讀 ref，不依賴 state
  const getRefreshTiming = useCallback(
    (subscriptionId: number): { interval: number; lastFetch: number } | undefined => {
      return refreshTimingRef.current.get(subscriptionId);
    },
    []
  );

  const setActiveSubIds = useCallback((ids: number[] | null) => {
    activeSubIdsRef.current = ids;
    startPolling(ids);
  }, [startPolling]);

  useEffect(() => {
    const init = async () => {
      loadProviderInfo();
      startPolling(null);
    };
    init();
    return () => {
      intervalsRef.current.forEach((interval) => clearInterval(interval));
      if (wsUnlistenRef.current) wsUnlistenRef.current();
    };
  }, []);

  return {
    subscriptions, providerInfoList, loading,
    addSubscription, removeSubscription, removeSubscriptions, updateSubscription,
    getAsset, getError, getSelectedProvider, getAssetType, getRefreshTiming,
    setActiveSubIds,
    refresh: startPolling,
  };
}
