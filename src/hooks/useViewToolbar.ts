import { useState, useCallback, useMemo } from 'react';
import type { View, ToastActions } from '../types';
import { t } from '../lib/i18n';

type EditorState = null | { mode: 'create' } | { mode: 'rename'; viewId: number; currentName: string };

interface UseViewToolbarOptions {
  views: View[];
  activeViewId: number;
  createView: (name: string) => Promise<void>;
  renameView: (viewId: number, name: string) => Promise<void>;
  deleteView: (viewId: number) => Promise<void>;
  toast: ToastActions;
  storageKey: string;
  /** 自訂確認刪除 — 回傳 true 表示確認 */
  confirmDelete?: (message: string) => Promise<boolean>;
}

export function useViewToolbar({
  views, activeViewId, createView, renameView, deleteView, toast, storageKey, confirmDelete,
}: UseViewToolbarOptions) {
  const [editorState, setEditorState] = useState<EditorState>(null);
  const [pinnedViewIds, setPinnedViewIds] = useState<number[]>(() => {
    try { return JSON.parse(localStorage.getItem(storageKey) || '[]'); } catch { return []; }
  });

  const handleCreateView = useCallback(() => setEditorState({ mode: 'create' }), []);

  const handleRequestRename = useCallback((viewId: number) => {
    const view = views.find(v => v.id === viewId);
    if (view) setEditorState({ mode: 'rename', viewId, currentName: view.name });
  }, [views]);

  const handleEditorConfirm = useCallback((name: string) => {
    if (!editorState) return;
    if (editorState.mode === 'create') {
      createView(name)
        .then(() => toast.success(t.views.created, t.views.viewCreated(name)))
        .catch(err => toast.error(t.views.createFailed, err instanceof Error ? err.message : String(err)));
    } else {
      renameView(editorState.viewId, name)
        .then(() => toast.success(t.views.renamed, t.views.viewRenamed(name)))
        .catch(err => toast.error(t.views.renameFailed, err instanceof Error ? err.message : String(err)));
    }
    setEditorState(null);
  }, [editorState, createView, renameView, toast]);

  const handleDeleteView = useCallback(async (viewId: number) => {
    const confirmed = confirmDelete
      ? await confirmDelete(t.views.deleteViewConfirm)
      : true;
    if (confirmed) {
      const viewName = views.find(v => v.id === viewId)?.name;
      deleteView(viewId)
        .then(() => toast.success(t.views.deleted, viewName ? t.views.viewDeleted(viewName) : ''))
        .catch(err => toast.error(t.views.deleteFailed, err instanceof Error ? err.message : String(err)));
      setPinnedViewIds(prev => {
        const next = prev.filter(id => id !== viewId);
        localStorage.setItem(storageKey, JSON.stringify(next));
        return next;
      });
    }
  }, [views, deleteView, toast, storageKey, confirmDelete]);

  const togglePinView = useCallback((viewId: number) => {
    setPinnedViewIds(prev => {
      const next = prev.includes(viewId) ? prev.filter(id => id !== viewId) : [...prev, viewId];
      localStorage.setItem(storageKey, JSON.stringify(next));
      return next;
    });
  }, [storageKey]);

  const sortedViews = useMemo(() =>
    [...views].sort((a, b) => {
      if (a.is_default) return -1;
      if (b.is_default) return 1;
      return a.id - b.id;
    }),
    [views]
  );

  const toolbarViews = useMemo(() => {
    if (sortedViews.length === 0) return [];
    const pinned = sortedViews.filter(v =>
      v.is_default || pinnedViewIds.includes(v.id) || v.id === activeViewId
    );
    const MAX_AUTO = 5;
    const hasPins = pinnedViewIds.some(pid => sortedViews.some(v => v.id === pid && !v.is_default));
    if (!hasPins && sortedViews.length > 1) {
      const auto = sortedViews.slice(0, MAX_AUTO);
      if (activeViewId && !auto.find(v => v.id === activeViewId)) {
        const activeView = sortedViews.find(v => v.id === activeViewId);
        if (activeView) auto.push(activeView);
      }
      return auto;
    }
    return pinned;
  }, [sortedViews, pinnedViewIds, activeViewId]);

  return {
    editorState,
    setEditorState,
    pinnedViewIds,
    toolbarViews,
    handleCreateView,
    handleRequestRename,
    handleEditorConfirm,
    handleDeleteView,
    togglePinView,
  };
}
