import { useState, useMemo, useCallback, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAssetData } from '../../hooks/useAssetData';
import { useViews } from '../../hooks/useViews';
import { useViewToolbar } from '../../hooks/useViewToolbar';
import { t } from '../../lib/i18n';
import { useLocale } from '../../hooks/useLocale';
import { DexCard } from './DexCard';
import { DexSubscriptionManager } from './DexSubscriptionManager';
import { ViewEditor } from '../ViewEditor/ViewEditor';
import { ViewSubscriptionManager } from '../ViewEditor/ViewSubscriptionManager';
import { ViewManager } from '../ViewManager/ViewManager';
import { BulkDelete } from '../BulkDelete/BulkDelete';

import './DexPage.css';

type ViewMode = 'grid' | 'list' | 'compact';

interface DexPageProps {
  onToast: {
    success: (title: string, msg?: string) => void;
    error: (title: string, msg?: string) => void;
    info: (title: string, msg?: string) => void;
  };
}

export function DexPage({ onToast }: DexPageProps) {
  useLocale();
  const [viewMode, setViewMode] = useState<ViewMode>(() => {
    const saved = localStorage.getItem('sb_dex_view_mode');
    if (saved === 'list' || saved === 'compact') return saved;
    return 'grid';
  });
  const [showAddSub, setShowAddSub] = useState(false);
  const [showSubManager, setShowSubManager] = useState(false);
  const [showBulkDelete, setShowBulkDelete] = useState(false);
  const [showViewManager, setShowViewManager] = useState(false);

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

  const {
    editorState, setEditorState, pinnedViewIds, toolbarViews,
    handleCreateView, handleRequestRename, handleEditorConfirm,
    handleDeleteView, togglePinView,
  } = useViewToolbar({
    views, activeViewId, createView, renameView, deleteView, toast: onToast,
    storageKey: 'sb_dex_pinned_views',
  });

  const isCustomView = activeViewSubscriptionIds !== null;

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
    invoke('set_visible_subscriptions', { ids, scope: 'dex' }).catch(err =>
      console.error('Failed to set visible subscriptions:', err)
    );
  }, [viewFilteredSubs]);

  const handleRemove = useCallback(async (id: number) => {
    if (isCustomView) {
      await removeSubscriptionFromView(activeViewId, id);
      onToast.info(t.subs.removedFromView, t.subs.removedFromViewMsg());
    } else {
      await removeSubscription(id);
      onToast.info(t.subs.unsubscribed, t.dex.unsubDex);
    }
  }, [removeSubscription, removeSubscriptionFromView, activeViewId, isCustomView, onToast]);

  const handleAdd = useCallback(async (pool: string, tf: string, tt: string, pid: string, dn?: string) => {
    await addDexSubscription(pool, tf, tt, pid, dn);
    await refreshViews();
  }, [addDexSubscription, refreshViews]);

  const handleCopySymbols = () => {
    const labels = viewFilteredSubs.map(s => s.display_name || s.symbol).join(', ');
    navigator.clipboard.writeText(labels).then(() => {
      onToast.success(t.common.copied, t.subs.pairsCopied(viewFilteredSubs.length));
    }).catch(() => {
      onToast.error(t.common.copyFailed);
    });
  };

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

  if (loading) return <div className="loading">{t.common.loading}</div>;

  return (
    <div className="dex-page">
      {subscriptions.length === 0 && !showAddSub ? (
        <div className="empty-state">
          <p>{t.subs.noDexSubs}</p>
          <button className="btn-add" onClick={() => setShowAddSub(true)}>{t.subs.addDexSub}</button>
        </div>
      ) : (
        <>
          <div className="dashboard-toolbar">
            <div className="dashboard-filters" role="tablist" aria-label={t.nav.dexPageSwitch}>
              {toolbarViews.map(view => (
                <button
                  key={view.id}
                  className={`view-tag ${view.id === activeViewId ? 'active' : ''} ${view.is_default ? 'default' : ''}`}
                  role="tab"
                  aria-selected={view.id === activeViewId}
                  onClick={() => setActiveView(view.id)}
                >
                  {view.is_default ? t.providers.all : view.name}
                  {view.is_default
                    ? ` (${subscriptions.length})`
                    : ` (${viewSubCounts[view.id] ?? 0})`
                  }
                </button>
              ))}
              {views.filter(v => !v.is_default).length > 0 && (
                <button className="view-manager-btn" onClick={() => setShowViewManager(true)} title={t.views.manageViews}>⋯</button>
              )}
              <button className="add-view-btn" onClick={handleCreateView} title={t.views.addView}>+</button>
            </div>
            <div className="toolbar-right">
              {activeViewSubscriptionIds !== null && (
                <button
                  className={`manage-subs-btn ${showSubManager ? 'active' : ''}`}
                  onClick={() => setShowSubManager(prev => !prev)}
                >
                  {t.subs.manageSubs}
                </button>
              )}
              <button className="add-sub-btn" onClick={() => setShowAddSub(true)} title={t.subs.addDexSub}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="16"/><line x1="8" y1="12" x2="16" y2="12"/></svg>
              </button>
              <button className="copy-symbols-btn" onClick={handleCopySymbols} title={t.subs.copyAllPairs}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
              </button>
              <div className="view-toggle">
                <button className={`view-btn ${viewMode === 'compact' ? 'active' : ''}`} onClick={() => handleSetViewMode('compact')} title={t.viewMode.compact}>▪</button>
                <button className={`view-btn ${viewMode === 'grid' ? 'active' : ''}`} onClick={() => handleSetViewMode('grid')} title={t.viewMode.grid}>▦</button>
                <button className={`view-btn ${viewMode === 'list' ? 'active' : ''}`} onClick={() => handleSetViewMode('list')} title={t.viewMode.list}>☰</button>
              </div>
              <button className="bulk-delete-btn" onClick={() => setShowBulkDelete(true)} title={isCustomView ? t.subs.bulkRemoveView : t.subs.bulkUnsubscribe}>
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
              onToast.info(t.subs.bulkRemoveView, t.subs.bulkRemovedView(count));
            } else {
              await removeSubscriptions([...ids]);
              setShowBulkDelete(false);
              onToast.info(t.subs.bulkUnsubscribe, t.subs.bulkUnsubscribed(count));
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
