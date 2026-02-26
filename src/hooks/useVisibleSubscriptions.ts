import { useMemo, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Subscription } from '../types';

/**
 * 根據 view 過濾訂閱，並同步 visible subscription ids 到 Rust 後端。
 * 消除 App.tsx 和 DexPage.tsx 的重複邏輯。
 */
export function useVisibleSubscriptions(
  subscriptions: Subscription[],
  activeViewSubscriptionIds: number[] | null,
  scope: 'asset' | 'dex',
) {
  const viewFilteredSubs = useMemo(() => {
    if (activeViewSubscriptionIds === null) return subscriptions;
    const idSet = new Set(activeViewSubscriptionIds);
    return subscriptions.filter(sub => idSet.has(sub.id));
  }, [subscriptions, activeViewSubscriptionIds]);

  const prevVisibleRef = useRef<string>('');
  useEffect(() => {
    const ids = viewFilteredSubs.map(s => s.id);
    const key = ids.join(',');
    if (key === prevVisibleRef.current) return;
    prevVisibleRef.current = key;
    invoke('set_visible_subscriptions', { ids, scope }).catch(() => {});
  }, [viewFilteredSubs, scope]);

  return viewFilteredSubs;
}
