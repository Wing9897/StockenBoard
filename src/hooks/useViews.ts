import { useState, useEffect, useCallback, useRef } from 'react';
import { View } from '../types';
import { getDb } from '../lib/db';

interface RawView { id: number; name: string; is_default: number }
const toView = (r: RawView): View => ({ id: r.id, name: r.name, is_default: r.is_default === 1 });

export function useViews() {
  const [views, setViews] = useState<View[]>([]);
  const [activeViewId, setActiveViewId] = useState<number>(() => {
    const saved = localStorage.getItem('sb_active_view_id');
    return saved ? parseInt(saved, 10) : -1;
  });
  const [activeViewSubscriptionIds, setActiveViewSubscriptionIds] = useState<number[] | null>(null);
  const [viewSubCounts, setViewSubCounts] = useState<Record<number, number>>({});
  const [loading, setLoading] = useState(true);

  const viewsRef = useRef<View[]>([]);
  viewsRef.current = views;
  const activeViewIdRef = useRef(activeViewId);
  activeViewIdRef.current = activeViewId;

  const loadViewSubCounts = useCallback(async (viewsList: View[]) => {
    try {
      const db = await getDb();
      const rows = await db.select<{ view_id: number; cnt: number }[]>(
        'SELECT view_id, COUNT(*) as cnt FROM view_subscriptions GROUP BY view_id'
      );
      const counts: Record<number, number> = {};
      for (const v of viewsList) if (!v.is_default) counts[v.id] = 0;
      for (const r of rows) counts[r.view_id] = r.cnt;
      setViewSubCounts(counts);
    } catch (err) {
      console.error('Failed to load view sub counts:', err);
    }
  }, []);

  const loadViews = useCallback(async () => {
    try {
      const db = await getDb();
      const rows = await db.select<RawView[]>('SELECT id, name, is_default FROM views ORDER BY id');
      const loaded = rows.map(toView);
      setViews(loaded);
      await loadViewSubCounts(loaded);
      return loaded;
    } catch (err) {
      console.error('Failed to load views:', err);
      return [];
    }
  }, [loadViewSubCounts]);

  const loadActiveViewSubs = useCallback(async (viewId: number, viewsList: View[]) => {
    const view = viewsList.find(v => v.id === viewId);
    if (!view || view.is_default) { setActiveViewSubscriptionIds(null); return; }
    try {
      const db = await getDb();
      const rows = await db.select<{ subscription_id: number }[]>(
        'SELECT subscription_id FROM view_subscriptions WHERE view_id = $1', [viewId]
      );
      setActiveViewSubscriptionIds(rows.map(r => r.subscription_id));
    } catch (err) {
      console.error('Failed to load view subscriptions:', err);
      setActiveViewSubscriptionIds(null);
    }
  }, []);

  const setActiveView = useCallback((viewId: number) => {
    setActiveViewId(viewId);
    localStorage.setItem('sb_active_view_id', String(viewId));
    loadActiveViewSubs(viewId, viewsRef.current);
  }, [loadActiveViewSubs]);

  const createView = useCallback(async (name: string) => {
    const trimmed = name.trim();
    if (!trimmed) throw new Error('View name cannot be empty');
    if (viewsRef.current.some(v => v.name.trim().toLowerCase() === trimmed.toLowerCase()))
      throw new Error('View name already exists');
    const db = await getDb();
    await db.execute('INSERT INTO views (name, is_default) VALUES ($1, 0)', [trimmed]);
    await loadViews();
  }, [loadViews]);

  const renameView = useCallback(async (viewId: number, newName: string) => {
    const view = viewsRef.current.find(v => v.id === viewId);
    if (!view) throw new Error('View not found');
    if (view.is_default) throw new Error('Cannot rename the default view');
    const trimmed = newName.trim();
    if (!trimmed) throw new Error('View name cannot be empty');
    if (viewsRef.current.some(v => v.id !== viewId && v.name.trim().toLowerCase() === trimmed.toLowerCase()))
      throw new Error('View name already exists');
    const db = await getDb();
    await db.execute('UPDATE views SET name = $1 WHERE id = $2', [trimmed, viewId]);
    await loadViews();
  }, [loadViews]);

  const deleteView = useCallback(async (viewId: number) => {
    const view = viewsRef.current.find(v => v.id === viewId);
    if (!view) throw new Error('View not found');
    if (view.is_default) throw new Error('Cannot delete the default view');
    const db = await getDb();
    await db.execute('DELETE FROM views WHERE id = $1', [viewId]);
    const updated = await loadViews();
    if (activeViewIdRef.current === viewId) {
      const def = updated.find(v => v.is_default);
      if (def) {
        setActiveViewId(def.id);
        localStorage.setItem('sb_active_view_id', String(def.id));
        setActiveViewSubscriptionIds(null);
      }
    }
  }, [loadViews]);

  const addSubscriptionToView = useCallback(async (viewId: number, subscriptionId: number) => {
    const db = await getDb();
    await db.execute('INSERT OR IGNORE INTO view_subscriptions (view_id, subscription_id) VALUES ($1, $2)', [viewId, subscriptionId]);
    if (viewId === activeViewIdRef.current) await loadActiveViewSubs(viewId, viewsRef.current);
    await loadViewSubCounts(viewsRef.current);
  }, [loadActiveViewSubs, loadViewSubCounts]);

  const removeSubscriptionFromView = useCallback(async (viewId: number, subscriptionId: number) => {
    const db = await getDb();
    await db.execute('DELETE FROM view_subscriptions WHERE view_id = $1 AND subscription_id = $2', [viewId, subscriptionId]);
    if (viewId === activeViewIdRef.current) await loadActiveViewSubs(viewId, viewsRef.current);
    await loadViewSubCounts(viewsRef.current);
  }, [loadActiveViewSubs, loadViewSubCounts]);

  useEffect(() => {
    (async () => {
      const loaded = await loadViews();
      const savedId = parseInt(localStorage.getItem('sb_active_view_id') || '', 10);
      const target = loaded.find(v => v.id === savedId) || loaded.find(v => v.is_default);
      if (target) {
        setActiveViewId(target.id);
        localStorage.setItem('sb_active_view_id', String(target.id));
        if (!target.is_default) await loadActiveViewSubs(target.id, loaded);
        else setActiveViewSubscriptionIds(null);
      }
      setLoading(false);
    })();
  }, []);

  return {
    views, activeViewId, activeViewSubscriptionIds, viewSubCounts, loading,
    setActiveView, createView, renameView, deleteView,
    addSubscriptionToView, removeSubscriptionFromView,
    refresh: loadViews,
  };
}
