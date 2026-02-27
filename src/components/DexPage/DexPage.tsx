import { useState, useMemo, useCallback } from 'react';
import { useAssetData } from '../../hooks/useAssetData';
import { useViews } from '../../hooks/useViews';
import { useViewToolbar } from '../../hooks/useViewToolbar';
import { useVisibleSubscriptions } from '../../hooks/useVisibleSubscriptions';
import { useBulkDelete } from '../../hooks/useBulkDelete';
import { t } from '../../lib/i18n';
import { getGridClass } from '../../lib/viewUtils';
import { useLocale } from '../../hooks/useLocale';
import { useConfirm } from '../../hooks/useConfirm';
import { DexCard } from './DexCard';
import { DexSubscriptionManager } from './DexSubscriptionManager';
import { ViewEditor } from '../ViewEditor/ViewEditor';
import { ViewSubscriptionManager } from '../ViewEditor/ViewSubscriptionManager';
import { ViewManager } from '../ViewManager/ViewManager';
import { BulkDelete } from '../BulkDelete/BulkDelete';
import { BatchActions } from '../BatchActions/BatchActions';
import { ConfirmDialog } from '../ConfirmDialog/ConfirmDialog';
import { DashboardToolbar } from '../DashboardToolbar/DashboardToolbar';

import type { ViewMode } from '../../types';
import './DexPage.css';

interface DexPageProps {
  onToast: {
    success: (title: string, msg?: string) => void;
    error: (title: string, msg?: string) => void;
    info: (title: string, msg?: string) => void;
  };
}

export function DexPage({ onToast }: DexPageProps) {
  useLocale();
  const { confirmState, requestConfirm, handleConfirm, handleCancel } = useConfirm();
  const [viewMode, setViewMode] = useState<ViewMode>(() => {
    const saved = localStorage.getItem('sb_dex_view_mode');
    if (saved === 'list' || saved === 'compact') return saved;
    return 'grid';
  });
  const [showAddSub, setShowAddSub] = useState(false);
  const [showSubManager, setShowSubManager] = useState(false);
  const [showBulkDelete, setShowBulkDelete] = useState(false);
  const [showViewManager, setShowViewManager] = useState(false);
  const [showBatchActions, setShowBatchActions] = useState(false);

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
    confirmDelete: requestConfirm,
  });

  const isCustomView = activeViewSubscriptionIds !== null;

  const viewFilteredSubs = useVisibleSubscriptions(subscriptions, activeViewSubscriptionIds, 'dex');

  const handleBulkConfirm = useBulkDelete({
    isCustomView, activeViewId,
    removeSubscriptions, removeSubscriptionFromView,
    requestConfirm, toast: onToast,
    onDone: () => setShowBulkDelete(false),
  });

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
      record_enabled: s.record_enabled ?? 0,
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
          <DashboardToolbar
            toolbarViews={toolbarViews}
            views={views}
            activeViewId={activeViewId}
            totalCount={subscriptions.length}
            viewSubCounts={viewSubCounts}
            isCustomView={isCustomView}
            showSubManager={showSubManager}
            viewMode={viewMode}
            onSetViewMode={handleSetViewMode}
            onSelectView={setActiveView}
            onCreateView={handleCreateView}
            onOpenViewManager={() => setShowViewManager(true)}
            onToggleSubManager={() => setShowSubManager(prev => !prev)}
            onAdd={() => setShowAddSub(true)}
            onCopy={handleCopySymbols}
            onBatchActions={() => setShowBatchActions(true)}
            onBulkDelete={() => setShowBulkDelete(true)}
            addTitle={t.subs.addDexSub}
            copyTitle={t.subs.copyAllPairs}
            bulkDeleteTitle={isCustomView ? t.subs.bulkRemoveView : t.subs.bulkUnsubscribe}
            tabListLabel={t.nav.dexPageSwitch}
          />

          <div className={getGridClass(viewMode)}>
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
          onConfirm={handleBulkConfirm}
          onClose={() => setShowBulkDelete(false)}
        />
      )}

      {showAddSub && (
        <DexSubscriptionManager
          onAdd={handleAdd}
          existingKeys={new Set(subscriptions.map(s => `${s.selected_provider_id}:${s.symbol}`))}
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

      {showBatchActions && (
        <BatchActions
          mode="dex"
          onClose={() => setShowBatchActions(false)}
        />
      )}

      {confirmState && (
        <ConfirmDialog message={confirmState.message} onConfirm={handleConfirm} onCancel={handleCancel} />
      )}
    </div>
  );
}
