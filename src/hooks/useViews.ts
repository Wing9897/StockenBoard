import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { View } from '../types';
import { silentLog } from '../lib/errorLog';
import { STORAGE_KEYS } from '../lib/storageKeys';

interface RawView { id: number; name: string; view_type: string; is_default: boolean }
const toView = (r: RawView): View => ({ id: r.id, name: r.name, view_type: r.view_type as 'asset' | 'dex', is_default: r.is_default });

export function useViews(viewType: 'asset' | 'dex' = 'asset') {
  const storageKey = viewType === 'dex' ? STORAGE_KEYS.DEX_ACTIVE_VIEW_ID : STORAGE_KEYS.ACTIVE_VIEW_ID;
  const [views, setViews] = useState<View[]>([]);
  const [activeViewId, setActiveViewId] = useState<number>(() => {
    const saved = localStorage.getItem(storageKey);
    return saved ? parseInt(saved, 10) : -1;
  });
  const [activeViewSubscriptionIds, setActiveViewSubscriptionIds] = useState<number[] | null>(null);
  const [viewSubCounts, setViewSubCounts] = useState<Record<number, number>>({});
  const [loading, setLoading] = useState(true);
  const viewsRef = useRef<View[]>([]);
  viewsRef.current = views;
  const activeViewIdRef = useRef(activeViewId);
  activeViewIdRef.current = activeViewId;

  const loadViewSubCounts = useCallback(async (vl: View[]) => {
    try {
      const rows = await invoke<{ view_id: number; count: number }[]>('get_view_sub_counts');
      const counts: Record<number, number> = {};
      for (const v of vl) if (!v.is_default) counts[v.id] = 0;
      for (const r of rows) counts[r.view_id] = r.count;
      setViewSubCounts(counts);
    } catch (e) { silentLog('loadViewSubCounts', e); }
  }, []);

  const loadViews = useCallback(async () => {
    try {
      const rows = await invoke<RawView[]>('list_views', { viewType });
      const loaded = rows.map(toView);
      setViews(loaded);
      await loadViewSubCounts(loaded);
      return loaded;
    } catch (e) { silentLog('loadViews', e); return []; }
  }, [viewType, loadViewSubCounts]);

  const loadActiveViewSubs = useCallback(async (viewId: number, vl: View[]) => {
    const view = vl.find(v => v.id === viewId);
    if (!view || view.is_default) { setActiveViewSubscriptionIds(null); return; }
    try {
      const ids = await invoke<number[]>('get_view_subscription_ids', { viewId });
      setActiveViewSubscriptionIds(ids);
    } catch (e) { silentLog('loadActiveViewSubs', e); setActiveViewSubscriptionIds(null); }
  }, []);

  const setActiveView = useCallback((viewId: number) => {
    setActiveViewId(viewId);
    localStorage.setItem(storageKey, String(viewId));
    loadActiveViewSubs(viewId, viewsRef.current);
  }, [storageKey, loadActiveViewSubs]);

  const createView = useCallback(async (name: string) => {
    const trimmed = name.trim();
    if (!trimmed) throw new Error('View name cannot be empty');
    if (viewsRef.current.some(v => v.name.trim().toLowerCase() === trimmed.toLowerCase())) throw new Error('View name already exists');
    await invoke<number>('create_view', { name: trimmed, viewType });
    await loadViews();
  }, [viewType, loadViews]);

  const renameView = useCallback(async (viewId: number, newName: string) => {
    const view = viewsRef.current.find(v => v.id === viewId);
    if (!view) throw new Error('View not found');
    if (view.is_default) throw new Error('Cannot rename the default view');
    const trimmed = newName.trim();
    if (!trimmed) throw new Error('View name cannot be empty');
    if (viewsRef.current.some(v => v.id !== viewId && v.name.trim().toLowerCase() === trimmed.toLowerCase())) throw new Error('View name already exists');
    await invoke('rename_view', { id: viewId, name: trimmed });
    await loadViews();
  }, [loadViews]);

  const deleteView = useCallback(async (viewId: number) => {
    const view = viewsRef.current.find(v => v.id === viewId);
    if (!view) throw new Error('View not found');
    if (view.is_default) throw new Error('Cannot delete the default view');
    await invoke('delete_view', { id: viewId });
    const updated = await loadViews();
    if (activeViewIdRef.current === viewId) {
      const def = updated.find(v => v.is_default);
      if (def) { setActiveViewId(def.id); localStorage.setItem(storageKey, String(def.id)); setActiveViewSubscriptionIds(null); }
    }
  }, [storageKey, loadViews]);

  const addSubscriptionToView = useCallback(async (viewId: number, subscriptionId: number) => {
    await invoke('add_sub_to_view', { viewId, subscriptionId });
    if (viewId === activeViewIdRef.current) await loadActiveViewSubs(viewId, viewsRef.current);
    await loadViewSubCounts(viewsRef.current);
  }, [loadActiveViewSubs, loadViewSubCounts]);

  const removeSubscriptionFromView = useCallback(async (viewId: number, subscriptionId: number) => {
    await invoke('remove_sub_from_view', { viewId, subscriptionId });
    if (viewId === activeViewIdRef.current) await loadActiveViewSubs(viewId, viewsRef.current);
    await loadViewSubCounts(viewsRef.current);
  }, [loadActiveViewSubs, loadViewSubCounts]);

  useEffect(() => {
    (async () => {
      const loaded = await loadViews();
      const savedId = parseInt(localStorage.getItem(storageKey) || '', 10);
      const target = loaded.find(v => v.id === savedId) || loaded.find(v => v.is_default);
      if (target) {
        setActiveViewId(target.id);
        localStorage.setItem(storageKey, String(target.id));
        if (!target.is_default) await loadActiveViewSubs(target.id, loaded);
        else setActiveViewSubscriptionIds(null);
      }
      setLoading(false);
    })();
  }, [loadViews, storageKey, loadActiveViewSubs]);

  return {
    views, activeViewId, activeViewSubscriptionIds, viewSubCounts, loading,
    setActiveView, createView, renameView, deleteView,
    addSubscriptionToView, removeSubscriptionFromView,
    refresh: loadViews,
  };
}
