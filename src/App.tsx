import { useState, useCallback, useRef, lazy, Suspense } from 'react';
import { useAssetData } from './hooks/useAssetData';
import { useViews } from './hooks/useViews';
import { useViewToolbar } from './hooks/useViewToolbar';
import { useToast } from './hooks/useToast';
import { useConfirm } from './hooks/useConfirm';
import { useEscapeKey } from './hooks/useEscapeKey';
import { useVisibleSubscriptions } from './hooks/useVisibleSubscriptions';
import { useBulkDelete } from './hooks/useBulkDelete';
import { useAppModals } from './hooks/useAppModals';
import { AssetCard } from './components/AssetCard/AssetCard';
import { ViewEditor } from './components/ViewEditor/ViewEditor';
import { ViewSubscriptionManager } from './components/ViewEditor/ViewSubscriptionManager';
import { ViewManager } from './components/ViewManager/ViewManager';
import { BulkDelete } from './components/BulkDelete/BulkDelete';
import { BatchActions } from './components/BatchActions/BatchActions';
import { ProviderSettings } from './components/Settings/ProviderSettings';
import { SubscriptionManager } from './components/Settings/SubscriptionManager';
import { DataManager } from './components/Settings/DataManager';
import { ThemePicker } from './components/Settings/ThemePicker';
import { LanguagePicker } from './components/Settings/LanguagePicker';
import { UICustomizer } from './components/Settings/UICustomizer';
import { ApiGuide } from './components/Settings/ApiGuide';
import { ConfirmDialog } from './components/ConfirmDialog/ConfirmDialog';
import { DashboardToolbar } from './components/DashboardToolbar/DashboardToolbar';
import { ToastContainer } from './components/Toast/Toast';
import { t } from './lib/i18n';
import { useLocale } from './hooks/useLocale';
import { getGridClass } from './lib/viewUtils';
import { STORAGE_KEYS } from './lib/storageKeys';
import type { ViewMode } from './types';
import './App.css';

// Lazy-loaded pages — 非活躍 tab 按需載入以減少初始 bundle
const DexPage = lazy(() => import('./components/DexPage/DexPage').then(m => ({ default: m.DexPage })));
const HistoryPage = lazy(() => import('./components/HistoryPage/HistoryPage').then(m => ({ default: m.HistoryPage })));

type Tab = 'dashboard' | 'dex' | 'history' | 'providers' | 'settings';

function App() {
  useLocale();
  const [activeTab, setActiveTabRaw] = useState<Tab>(() => {
    const saved = localStorage.getItem(STORAGE_KEYS.ACTIVE_TAB) as Tab | null;
    if (saved === 'dashboard' || saved === 'dex' || saved === 'history' || saved === 'providers' || saved === 'settings') return saved;
    return 'dashboard';
  });
  const setActiveTab = useCallback((tab: Tab) => { setActiveTabRaw(tab); localStorage.setItem(STORAGE_KEYS.ACTIVE_TAB, tab); }, []);
  const [viewMode, setViewMode] = useState<ViewMode>(() => {
    const saved = localStorage.getItem(STORAGE_KEYS.VIEW_MODE);
    if (saved === 'list' || saved === 'compact') return saved;
    return 'grid';
  });
  const m = useAppModals();
  const [forceExpandAll, setForceExpandAll] = useState(() => localStorage.getItem(STORAGE_KEYS.EXPAND_ALL) === '1');
  const [hidePrePost, setHidePrePost] = useState(() => localStorage.getItem(STORAGE_KEYS.HIDE_PREPOST) === '1');
  const toast = useToast();
  const { confirmState, requestConfirm, handleConfirm, handleCancel } = useConfirm();

  const handleSetViewMode = (mode: ViewMode) => { setViewMode(mode); localStorage.setItem(STORAGE_KEYS.VIEW_MODE, mode); };

  const {
    views, activeViewId, activeViewSubscriptionIds, viewSubCounts,
    setActiveView, createView, renameView, deleteView,
    addSubscriptionToView, removeSubscriptionFromView, refresh: refreshViews,
  } = useViews('asset');

  const {
    editorState, setEditorState, pinnedViewIds, toolbarViews,
    handleCreateView, handleRequestRename, handleEditorConfirm,
    handleDeleteView, togglePinView,
  } = useViewToolbar({
    views, activeViewId, createView, renameView, deleteView, toast,
    storageKey: STORAGE_KEYS.PINNED_VIEWS, confirmDelete: requestConfirm,
  });

  const {
    subscriptions, providerInfoList, loading,
    addSubscription, addSubscriptionBatch,
    removeSubscription, removeSubscriptions,
    updateSubscription, getSelectedProvider, getAssetType, getRefreshInterval,
    refresh: refreshAssets,
  } = useAssetData();

  const handleAdd = useCallback(async (symbol: string, providerId?: string, assetType?: 'crypto' | 'stock') => {
    await addSubscription(symbol, undefined, providerId, assetType);
    await refreshViews();
  }, [addSubscription, refreshViews]);

  const handleBatchAdd = useCallback(async (
    items: { symbol: string; providerId?: string; assetType?: string }[],
    onProgress?: (done: number, total: number) => void,
  ) => {
    const result = await addSubscriptionBatch(items, onProgress);
    await refreshViews();
    return result;
  }, [addSubscriptionBatch, refreshViews]);

  const subscriptionsRef = useRef(subscriptions);
  subscriptionsRef.current = subscriptions;
  const isCustomView = activeViewSubscriptionIds !== null;

  const handleRemove = useCallback(async (id: number) => {
    const sub = subscriptionsRef.current.find(s => s.id === id);
    if (isCustomView) {
      await removeSubscriptionFromView(activeViewId, id);
      toast.info(t.subs.removedFromView, t.subs.removedFromViewMsg(sub?.symbol));
    } else {
      await removeSubscription(id);
      toast.info(t.subs.unsubscribed, t.subs.unsubscribedMsg(sub?.symbol));
    }
  }, [removeSubscription, removeSubscriptionFromView, activeViewId, isCustomView, toast]);

  const viewFilteredSubs = useVisibleSubscriptions(subscriptions, activeViewSubscriptionIds, 'asset');

  const handleBulkConfirm = useBulkDelete({
    isCustomView, activeViewId,
    removeSubscriptions, removeSubscriptionFromView,
    requestConfirm, toast,
    onDone: () => m.setShowBulkDelete(false),
  });

  const handleCopySymbols = () => {
    const symbols = viewFilteredSubs.map(s => s.symbol).join(', ');
    navigator.clipboard.writeText(symbols).then(() => {
      toast.success(t.common.copied, t.subs.symbolsCopied(viewFilteredSubs.length));
    }).catch(() => { toast.error(t.common.copyFailed); });
  };

  useEscapeKey(() => { if (m.showAddSubscription) m.setShowAddSubscription(false); });

  return (
    <div className="app">
      <ToastContainer toasts={toast.toasts} onRemove={toast.removeToast} />

      <header className="app-header">
        <h1>StockenBoard</h1>
        <nav className="app-nav">
          <button className={`nav-btn ${activeTab === 'dashboard' ? 'active' : ''}`} onClick={() => setActiveTab('dashboard')}>{t.nav.dashboard}</button>
          <button className={`nav-btn ${activeTab === 'dex' ? 'active' : ''}`} onClick={() => setActiveTab('dex')}>{t.nav.dex}</button>
          <button className={`nav-btn ${activeTab === 'history' ? 'active' : ''}`} onClick={() => setActiveTab('history')}>{t.nav.history}</button>
          <button className={`nav-btn ${activeTab === 'providers' ? 'active' : ''}`} onClick={() => setActiveTab('providers')}>{t.nav.providers}</button>
          <button className={`nav-btn ${activeTab === 'settings' ? 'active' : ''}`} onClick={() => setActiveTab('settings')}>{t.nav.settings}</button>
        </nav>
      </header>

      <main className="app-main">
        {activeTab === 'dashboard' && (
          <div className="dashboard">
            {loading ? (
              <div className="loading">{t.common.loading}</div>
            ) : subscriptions.length === 0 ? (
              <div className="empty-state">
                <p>{t.subs.noSubs}</p>
                <button className="btn-add" onClick={() => m.setShowAddSubscription(true)}>{t.subs.addSub}</button>
              </div>
            ) : (
              <>
                <DashboardToolbar
                  toolbarViews={toolbarViews} views={views} activeViewId={activeViewId}
                  totalCount={subscriptions.length} viewSubCounts={viewSubCounts}
                  isCustomView={isCustomView} showSubManager={m.showSubscriptionManager}
                  viewMode={viewMode} onSetViewMode={handleSetViewMode}
                  onSelectView={setActiveView} onCreateView={handleCreateView}
                  onOpenViewManager={() => m.setShowViewManager(true)}
                  onToggleSubManager={() => m.setShowSubscriptionManager(prev => !prev)}
                  onAdd={() => m.setShowAddSubscription(true)}
                  onCopy={handleCopySymbols}
                  onBatchActions={() => m.setShowBatchActions(true)}
                  onBulkDelete={() => m.setShowBulkDelete(true)}
                  addTitle={t.subs.addSub} copyTitle={t.subs.copyAllSymbols}
                  bulkDeleteTitle={isCustomView ? t.subs.bulkRemoveView : t.subs.bulkUnsubscribe}
                  tabListLabel={t.nav.pageSwitch}
                />
                <div className={getGridClass(viewMode)}>
                  {viewFilteredSubs.map(sub => (
                    <AssetCard key={sub.id} subscription={sub} providers={providerInfoList}
                      currentProviderId={getSelectedProvider(sub.id)} assetType={getAssetType(sub.id)}
                      refreshInterval={getRefreshInterval(sub.selected_provider_id)}
                      onRemove={handleRemove} onEdit={updateSubscription} viewMode={viewMode}
                      isCustomView={isCustomView} forceExpand={forceExpandAll} hidePrePost={hidePrePost}
                    />
                  ))}
                </div>
              </>
            )}
          </div>
        )}

        {activeTab === 'dex' && <Suspense fallback={<div className="loading">{t.common.loading}</div>}><DexPage onToast={toast} /></Suspense>}
        {activeTab === 'history' && <Suspense fallback={<div className="loading">{t.common.loading}</div>}><HistoryPage onToast={toast} /></Suspense>}
        {activeTab === 'providers' && (
          <div className="providers-page">
            <ProviderSettings onSaved={() => toast.success(t.providers.settingsSaved)} />
          </div>
        )}
        {activeTab === 'settings' && (
          <div className="settings">
            <ThemePicker />
            <LanguagePicker />
            <UICustomizer />
            <ApiGuide />
            <DataManager views={views} onRefresh={() => { refreshAssets(); refreshViews(); }}
              onToast={(type, title, msg) => toast[type](title, msg)} />
            <p className="settings-disclaimer">{t.settings.disclaimer}</p>
          </div>
        )}
      </main>

      {editorState && (
        <ViewEditor mode={editorState.mode}
          currentName={editorState.mode === 'rename' ? editorState.currentName : undefined}
          existingNames={views.map(v => v.name)}
          onConfirm={handleEditorConfirm} onCancel={() => setEditorState(null)} />
      )}

      {m.showSubscriptionManager && activeViewSubscriptionIds !== null && (
        <ViewSubscriptionManager allSubscriptions={subscriptions} viewSubscriptionIds={activeViewSubscriptionIds}
          onToggleSubscription={(subId, add) => { if (add) addSubscriptionToView(activeViewId, subId); else removeSubscriptionFromView(activeViewId, subId); }}
          onClose={() => m.setShowSubscriptionManager(false)} />
      )}

      {m.showViewManager && (
        <ViewManager views={views} activeViewId={activeViewId} pinnedViewIds={pinnedViewIds}
          onSelectView={setActiveView} onTogglePin={togglePinView} onRename={handleRequestRename}
          onDelete={handleDeleteView} onCreate={handleCreateView}
          onClose={() => m.setShowViewManager(false)} />
      )}

      {m.showBulkDelete && (
        <BulkDelete subscriptions={viewFilteredSubs} isCustomView={isCustomView}
          onConfirm={handleBulkConfirm} onClose={() => m.setShowBulkDelete(false)} />
      )}

      {m.showAddSubscription && (
        <div className="modal-backdrop" onClick={() => m.setShowAddSubscription(false)}>
          <div className="modal-container sub-modal" role="dialog" aria-modal="true" aria-label={t.subs.addSub} onClick={e => e.stopPropagation()}>
            <div className="sub-modal-header">
              <h4 className="sub-modal-title">{t.subs.addSub}</h4>
              <button className="vsm-close" onClick={() => m.setShowAddSubscription(false)} aria-label={t.common.close}>✕</button>
            </div>
            <div className="sub-modal-body">
              <SubscriptionManager onBatchAdd={handleAdd} onBatchAddMultiple={handleBatchAdd}
                subscriptions={subscriptions} providers={providerInfoList}
                onToast={(type, title, msg) => toast[type](title, msg)}
                onDone={() => m.setShowAddSubscription(false)} />
            </div>
          </div>
        </div>
      )}

      {m.showBatchActions && (
        <BatchActions mode="spot" expandAll={forceExpandAll} showPrePost={!hidePrePost}
          onToggleExpandAll={() => setForceExpandAll(v => { const next = !v; localStorage.setItem(STORAGE_KEYS.EXPAND_ALL, next ? '1' : '0'); return next; })}
          onTogglePrePost={() => setHidePrePost(v => { const next = !v; localStorage.setItem(STORAGE_KEYS.HIDE_PREPOST, next ? '1' : '0'); return next; })}
          onClose={() => m.setShowBatchActions(false)} />
      )}

      {confirmState && <ConfirmDialog message={confirmState.message} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  );
}

export default App;
