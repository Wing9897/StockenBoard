import { useState, useEffect, useCallback, useRef } from 'react';
import Database from '@tauri-apps/plugin-sql';
import { View } from '../types';

interface UseViewsReturn {
  views: View[];
  activeViewId: number;
  activeViewSubscriptionIds: number[] | null;
  viewSubCounts: Record<number, number>;
  loading: boolean;
  setActiveView: (viewId: number) => void;
  createView: (name: string) => Promise<void>;
  renameView: (viewId: number, newName: string) => Promise<void>;
  deleteView: (viewId: number) => Promise<void>;
  addSubscriptionToView: (viewId: number, subscriptionId: number) => Promise<void>;
  removeSubscriptionFromView: (viewId: number, subscriptionId: number) => Promise<void>;
  refresh: () => Promise<View[]>;
}

interface RawView {
  id: number;
  name: string;
  is_default: number;
}

function toView(raw: RawView): View {
  return {
    id: raw.id,
    name: raw.name,
    is_default: raw.is_default === 1,
  };
}

export function useViews(): UseViewsReturn {
  const [views, setViews] = useState<View[]>([]);
  const [activeViewId, setActiveViewId] = useState<number>(() => {
    const saved = localStorage.getItem('sb_active_view_id');
    return saved ? parseInt(saved, 10) : -1;
  });
  const [activeViewSubscriptionIds, setActiveViewSubscriptionIds] = useState<number[] | null>(null);
  const [viewSubCounts, setViewSubCounts] = useState<Record<number, number>>({});
  const [loading, setLoading] = useState(true);

  const loadViewSubCounts = useCallback(async (viewsList: View[]) => {
    try {
      const db = await Database.load('sqlite:stockenboard.db');
      const rows = await db.select<{ view_id: number; cnt: number }[]>(
        'SELECT view_id, COUNT(*) as cnt FROM view_subscriptions GROUP BY view_id'
      );
      const counts: Record<number, number> = {};
      for (const v of viewsList) {
        if (!v.is_default) counts[v.id] = 0;
      }
      for (const r of rows) {
        counts[r.view_id] = r.cnt;
      }
      setViewSubCounts(counts);
    } catch (err) {
      console.error('Failed to load view sub counts:', err);
    }
  }, []);

  const loadViews = useCallback(async () => {
    try {
      const db = await Database.load('sqlite:stockenboard.db');
      const rows = await db.select<RawView[]>(
        'SELECT id, name, is_default FROM views ORDER BY id'
      );
      const loaded = rows.map(toView);
      setViews(loaded);
      await loadViewSubCounts(loaded);
      return loaded;
    } catch (err) {
      console.error('Failed to load views:', err);
      return [];
    }
  }, [loadViewSubCounts]);

  const loadActiveViewSubscriptions = useCallback(async (viewId: number, viewsList: View[]) => {
    const view = viewsList.find((v) => v.id === viewId);
    if (!view || view.is_default) {
      setActiveViewSubscriptionIds(null);
      return;
    }
    try {
      const db = await Database.load('sqlite:stockenboard.db');
      const rows = await db.select<{ subscription_id: number }[]>(
        'SELECT subscription_id FROM view_subscriptions WHERE view_id = $1',
        [viewId]
      );
      setActiveViewSubscriptionIds(rows.map((r) => r.subscription_id));
    } catch (err) {
      console.error('Failed to load view subscriptions:', err);
      setActiveViewSubscriptionIds(null);
    }
  }, []);

  const viewsRef = useRef<View[]>([]);
  viewsRef.current = views;

  const setActiveView = useCallback(
    (viewId: number) => {
      setActiveViewId(viewId);
      localStorage.setItem('sb_active_view_id', String(viewId));
      loadActiveViewSubscriptions(viewId, viewsRef.current);
    },
    [loadActiveViewSubscriptions]
  );

  const createView = useCallback(async (name: string) => {
    const trimmed = name.trim();
    if (!trimmed) {
      throw new Error('View name cannot be empty');
    }

    const currentViews = viewsRef.current;
    const duplicate = currentViews.some(
      (v) => v.name.trim().toLowerCase() === trimmed.toLowerCase()
    );
    if (duplicate) {
      throw new Error('View name already exists');
    }

    try {
      const db = await Database.load('sqlite:stockenboard.db');
      await db.execute(
        'INSERT INTO views (name, is_default) VALUES ($1, 0)',
        [trimmed]
      );
      await loadViews();
    } catch (err) {
      console.error('Failed to create view:', err);
      throw err;
    }
  }, [loadViews]);

  const renameView = useCallback(async (viewId: number, newName: string) => {
    const currentViews = viewsRef.current;
    const view = currentViews.find((v) => v.id === viewId);
    if (!view) {
      throw new Error('View not found');
    }
    if (view.is_default) {
      throw new Error('Cannot rename the default view');
    }

    const trimmed = newName.trim();
    if (!trimmed) {
      throw new Error('View name cannot be empty');
    }

    const duplicate = currentViews.some(
      (v) => v.id !== viewId && v.name.trim().toLowerCase() === trimmed.toLowerCase()
    );
    if (duplicate) {
      throw new Error('View name already exists');
    }

    try {
      const db = await Database.load('sqlite:stockenboard.db');
      await db.execute('UPDATE views SET name = $1 WHERE id = $2', [trimmed, viewId]);
      await loadViews();
    } catch (err) {
      console.error('Failed to rename view:', err);
      throw err;
    }
  }, [loadViews]);

  const activeViewIdRef = useRef(activeViewId);
  activeViewIdRef.current = activeViewId;

  const deleteView = useCallback(async (viewId: number) => {
    const currentViews = viewsRef.current;
    const view = currentViews.find((v) => v.id === viewId);
    if (!view) {
      throw new Error('View not found');
    }
    if (view.is_default) {
      throw new Error('Cannot delete the default view');
    }

    try {
      const db = await Database.load('sqlite:stockenboard.db');
      await db.execute('DELETE FROM views WHERE id = $1', [viewId]);
      const updated = await loadViews();

      // If the deleted view was active, switch to default view
      if (activeViewIdRef.current === viewId) {
        const defaultView = updated.find((v) => v.is_default);
        if (defaultView) {
          setActiveViewId(defaultView.id);
          localStorage.setItem('sb_active_view_id', String(defaultView.id));
          setActiveViewSubscriptionIds(null);
        }
      }
    } catch (err) {
      console.error('Failed to delete view:', err);
      throw err;
    }
  }, [loadViews]);

  const addSubscriptionToView = useCallback(async (viewId: number, subscriptionId: number) => {
    try {
      const db = await Database.load('sqlite:stockenboard.db');
      await db.execute(
        'INSERT OR IGNORE INTO view_subscriptions (view_id, subscription_id) VALUES ($1, $2)',
        [viewId, subscriptionId]
      );
      if (viewId === activeViewIdRef.current) {
        await loadActiveViewSubscriptions(viewId, viewsRef.current);
      }
      await loadViewSubCounts(viewsRef.current);
    } catch (err) {
      console.error('Failed to add subscription to view:', err);
      throw err;
    }
  }, [loadActiveViewSubscriptions, loadViewSubCounts]);

  const removeSubscriptionFromView = useCallback(async (viewId: number, subscriptionId: number) => {
    try {
      const db = await Database.load('sqlite:stockenboard.db');
      await db.execute(
        'DELETE FROM view_subscriptions WHERE view_id = $1 AND subscription_id = $2',
        [viewId, subscriptionId]
      );
      if (viewId === activeViewIdRef.current) {
        await loadActiveViewSubscriptions(viewId, viewsRef.current);
      }
      await loadViewSubCounts(viewsRef.current);
    } catch (err) {
      console.error('Failed to remove subscription from view:', err);
      throw err;
    }
  }, [loadActiveViewSubscriptions, loadViewSubCounts]);

  // Initialize: load views and restore active view
  useEffect(() => {
    (async () => {
      const loaded = await loadViews();
      const savedId = parseInt(localStorage.getItem('sb_active_view_id') || '', 10);
      const savedView = loaded.find((v) => v.id === savedId);
      const targetView = savedView || loaded.find((v) => v.is_default);
      if (targetView) {
        setActiveViewId(targetView.id);
        localStorage.setItem('sb_active_view_id', String(targetView.id));
        if (!targetView.is_default) {
          await loadActiveViewSubscriptions(targetView.id, loaded);
        } else {
          setActiveViewSubscriptionIds(null);
        }
      }
      setLoading(false);
    })();
  }, []);

  return {
    views,
    activeViewId,
    activeViewSubscriptionIds,
    viewSubCounts,
    loading,
    setActiveView,
    createView,
    renameView,
    deleteView,
    addSubscriptionToView,
    removeSubscriptionFromView,
    refresh: loadViews,
  };
}
