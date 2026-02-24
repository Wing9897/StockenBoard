import { useState, useMemo, useCallback, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAssetData } from '../../hooks/useAssetData';
import { useViews } from '../../hooks/useViews';
import { DexCard } from './DexCard';
import { DexSubscriptionManager } from './DexSubscriptionManager';
import { ViewEditor } from '../ViewEditor/ViewEditor';
import { ViewSubscriptionManager } from '../ViewEditor/ViewSubscriptionManager';
import { ViewManager } from '../ViewManager/ViewManager';
import { BulkDelete } from '../BulkDelete/BulkDelete';

import './DexPage.css';

type ViewMode = 'grid' | 'list' | 'compact';
type EditorState = null | { mode: 'create' } | { mode: 'rename'; viewId: number; currentName: string };

interface DexPageProps {
  onToast: {
    success: (title: string, msg?: string) => void;
    error: (title: string, msg?: string) => void;
    info: (title: string, msg?: string) => void;
  };
}

export function DexPage({ onToast }: DexPageProps) {
  const [viewMode, setViewMode] = useState<ViewMode>(() => {
    const saved = localStorage.getItem('sb_dex_view_mode');
    if (saved === 'list' || saved === 'compact') return saved;
    return 'grid';
  });
  const [editorState, setEditorState] = useState<EditorState>(null);
  const [showAddSub, setShowAddSub] = useState(false);
  const [showSubManager, setShowSubManager] = useState(false);
  const [showBulkDelete, setShowBulkDelete] = useState(false);
  const [showViewManager, setShowViewManager] = useState(false);
  const [pinnedViewIds, setPinnedViewIds] = useState<number[]>(() => {
    try { return JSON.parse(localStorage.getItem('sb_dex_pinned_views') || '[]'); } catch { return []; }
  });

  const handleSetViewMode = (mode: ViewMode) => {
    setViewMode(mode);
    localStorage.setItem('sb_dex_view_mode', mode);
  };

  const {
    subscriptions, providerInfoList, loading,
    addDexSubscription, removeSubscription, removeSubscriptions,
    updateDexSubscription, getDexSymbol, getRefreshInterval,
  } = useAssetData('dex');

  const {
    views, activeViewId, activeViewSubscriptionIds, viewSubCounts,
    setActiveView, createView, renameView, deleteView,
    addSubscriptionToView, removeSubscriptionFromView,
    refresh: refreshViews,
  } = useViews('dex');

  const isCustomView = activeViewSubscriptionIds !== null;

  const viewFilteredSubs = useMemo(() => {
    if (activeViewSubscriptionIds === null) return subscriptions;
    const idSet = new Set(activeViewSubscriptionIds);
    return subscriptions.filter(sub => idSet.has(sub.id));
  }, [subscriptions, activeViewSubscriptionIds]);

  // Notify backend of visible subscription IDs
  const prevVisibleRef = useRef<string>('');
  useEffect(() => {
    const ids = viewFilteredSubs.map(s => s.id);
    const key = ids.join(',');
    if (key === prevVisibleRef.current) return;
    prevVisibleRef.current = key;
    invoke('set_visible_subscriptions', { ids, scope: 'dex' }).catch(err =>
      console.error('Failed to set visible subscriptions:', err)
    );
  }, [viewFilteredSubs]);

  const handleRemove = useCallback(async (id: number) => {
    if (isCustomView) {
      await removeSubscriptionFromView(activeViewId, id);
      onToast.info('已移除顯示', '已從此頁面移除');
    } else {
      await removeSubscription(id);
      onToast.info('已取消訂閱', '已取消 DEX 訂閱');
    }
  }, [removeSubscription, removeSubscriptionFromView, activeViewId, isCustomView, onToast]);

  const handleAdd = useCallback(async (pool: string, tf: string, tt: string, pid: string, dn?: string) => {
    await addDexSubscription(pool, tf, tt, pid, dn);
    await refreshViews();
  }, [addDexSubscription, refreshViews]);

  const handleCreateView = () => setEditorState({ mode: 'create' });
  const handleRequestRename = (viewId: number) => {
    const view = views.find(v => v.id === viewId);
    if (view) setEditorState({ mode: 'rename', viewId, currentName: view.name });
  };
  const handleEditorConfirm = (name: string) => {
    if (!editorState) return;
    if (editorState.mode === 'create') {
      createView(name)
        .then(() => onToast.success('已建立', `頁面「${name}」已建立`))
        .catch(err => onToast.error('建立頁面失敗', err instanceof Error ? err.message : String(err)));
    } else {
      renameView(editorState.viewId, name)
        .then(() => onToast.success('已重新命名', `頁面已更名為「${name}」`))
        .catch(err => onToast.error('重新命名失敗', err instanceof Error ? err.message : String(err)));
    }
    setEditorState(null);
  };
  const handleDeleteView = (viewId: number) => {
    if (confirm('確定要刪除此頁面嗎？')) {
      deleteView(viewId)
        .then(() => onToast.success('已刪除'))
        .catch(err => onToast.error('刪除失敗', err instanceof Error ? err.message : String(err)));
      setPinnedViewIds(prev => {
        const next = prev.filter(id => id !== viewId);
        localStorage.setItem('sb_dex_pinned_views', JSON.stringify(next));
        return next;
      });
    }
  };

  const togglePinView = (viewId: number) => {
    setPinnedViewIds(prev => {
      const next = prev.includes(viewId) ? prev.filter(id => id !== viewId) : [...prev, viewId];
      localStorage.setItem('sb_dex_pinned_views', JSON.stringify(next));
      return next;
    });
  };

  const handleCopySymbols = () => {
    const labels = viewFilteredSubs.map(s => s.display_name || s.symbol).join(', ');
    navigator.clipboard.writeText(labels).then(() => {
      onToast.success('已複製', `${viewFilteredSubs.length} 個交易對已複製到剪貼簿`);
    }).catch(() => {
      onToast.error('複製失敗');
    });
  };

  const sortedViews = [...views].sort((a, b) => {
    if (a.is_default) return -1;
    if (b.is_default) return 1;
    return a.id - b.id;
  });

  const toolbarViews = (() => {
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
  })();

  // Convert for ViewSubscriptionManager
  const subsForViewManager = useMemo(() =>
    subscriptions.map(s => ({
      id: s.id,
      sub_type: s.sub_type,
      symbol: s.display_name || `${(s.token_from_address || '').slice(0, 8)}→${(s.token_to_address || '').slice(0, 8)}`,
      display_name: s.display_name,
      selected_provider_id: s.selected_provider_id,
      asset_type: s.asset_type,
      sort_order: s.sort_order,
    })),
    [subscriptions]
  );

  if (loading) return <div className="loading">載入中...</div>;

  return (
    <div className="dex-page">
      {subscriptions.length === 0 && !showAddSub ? (
        <div className="empty-state">
          <p>尚未訂閱任何 DEX 交易對</p>
          <button className="btn-add" onClick={() => setShowAddSub(true)}>新增 DEX 訂閱</button>
        </div>
      ) : (
        <>
          <div className="dashboard-toolbar">
            <div className="dashboard-filters" role="tablist" aria-label="DEX 頁面切換">
              {toolbarViews.map(view => (
                <button
                  key={view.id}
                  className={`view-tag ${view.id === activeViewId ? 'active' : ''} ${view.is_default ? 'default' : ''}`}
                  role="tab"
                  aria-selected={view.id === activeViewId}
                  onClick={() => setActiveView(view.id)}
                >
                  {view.name}
                  {view.is_default
                    ? ` (${subscriptions.length})`
                    : ` (${viewSubCounts[view.id] ?? 0})`
                  }
                </button>
              ))}
              {views.filter(v => !v.is_default).length > 0 && (
                <button className="view-manager-btn" onClick={() => setShowViewManager(true)} title="管理頁面">⋯</button>
              )}
              <button className="add-view-btn" onClick={handleCreateView} title="新增頁面">+</button>
            </div>
            <div className="toolbar-right">
              {activeViewSubscriptionIds !== null && (
                <button
                  className={`manage-subs-btn ${showSubManager ? 'active' : ''}`}
                  onClick={() => setShowSubManager(prev => !prev)}
                >
                  管理訂閱
                </button>
              )}
              <button className="add-sub-btn" onClick={() => setShowAddSub(true)} title="新增 DEX 訂閱">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="16"/><line x1="8" y1="12" x2="16" y2="12"/></svg>
              </button>
              <button className="copy-symbols-btn" onClick={handleCopySymbols} title="複製所有交易對">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
              </button>
              <div className="view-toggle">
                <button className={`view-btn ${viewMode === 'compact' ? 'active' : ''}`} onClick={() => handleSetViewMode('compact')} title="小方塊">▪</button>
                <button className={`view-btn ${viewMode === 'grid' ? 'active' : ''}`} onClick={() => handleSetViewMode('grid')} title="方塊顯示">▦</button>
                <button className={`view-btn ${viewMode === 'list' ? 'active' : ''}`} onClick={() => handleSetViewMode('list')} title="列表顯示">☰</button>
              </div>
              <button className="bulk-delete-btn" onClick={() => setShowBulkDelete(true)} title={isCustomView ? '批量移除顯示' : '批量取消訂閱'}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
              </button>
            </div>
          </div>

          <div className={viewMode === 'grid' ? 'asset-grid' : viewMode === 'compact' ? 'asset-grid compact' : 'asset-list'}>
            {viewFilteredSubs.map(sub => (
              <DexCard
                key={sub.id}
                subscription={sub}
                providers={providerInfoList}
                refreshInterval={getRefreshInterval(sub.selected_provider_id)}
                onRemove={handleRemove}
                onEdit={updateDexSubscription}
                viewMode={viewMode}
                isCustomView={isCustomView}
                getDexSymbol={getDexSymbol}
              />
            ))}
          </div>
        </>
      )}

      {editorState && (
        <ViewEditor
          mode={editorState.mode}
          currentName={editorState.mode === 'rename' ? editorState.currentName : undefined}
          existingNames={views.map(v => v.name)}
          onConfirm={handleEditorConfirm}
          onCancel={() => setEditorState(null)}
        />
      )}

      {showSubManager && activeViewSubscriptionIds !== null && (
        <ViewSubscriptionManager
          allSubscriptions={subsForViewManager}
          viewSubscriptionIds={activeViewSubscriptionIds}
          onToggleSubscription={(subId, add) => {
            if (add) addSubscriptionToView(activeViewId, subId);
            else removeSubscriptionFromView(activeViewId, subId);
          }}
          onClose={() => setShowSubManager(false)}
        />
      )}

      {showBulkDelete && (
        <BulkDelete
          subscriptions={viewFilteredSubs}
          isCustomView={isCustomView}
          getLabel={(sub) => ({
            primary: sub.display_name || `${(sub.pool_address || '').slice(0, 10)}...`,
            secondary: sub.selected_provider_id,
          })}
          onConfirm={async (ids) => {
            if (ids.size === 0) return;
            const count = ids.size;
            if (isCustomView) {
              for (const id of ids) {
                await removeSubscriptionFromView(activeViewId, id);
              }
              setShowBulkDelete(false);
              onToast.info('批量移除顯示', `已從此頁面移除 ${count} 個項目`);
            } else {
              await removeSubscriptions([...ids]);
              setShowBulkDelete(false);
              onToast.info('批量取消訂閱', `已取消 ${count} 個訂閱`);
            }
          }}
          onClose={() => setShowBulkDelete(false)}
        />
      )}

      {showAddSub && (
        <DexSubscriptionManager
          onAdd={handleAdd}
          onToast={(type, title, msg) => onToast[type](title, msg)}
          onClose={() => setShowAddSub(false)}
        />
      )}

      {showViewManager && (
        <ViewManager
          views={views}
          activeViewId={activeViewId}
          pinnedViewIds={pinnedViewIds}
          onSelectView={setActiveView}
          onTogglePin={togglePinView}
          onRename={handleRequestRename}
          onDelete={handleDeleteView}
          onCreate={handleCreateView}
          onClose={() => setShowViewManager(false)}
        />
      )}
    </div>
  );
}
