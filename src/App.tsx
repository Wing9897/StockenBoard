import { useState, useMemo, useCallback, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAssetData } from './hooks/useAssetData';
import { useViews } from './hooks/useViews';
import { useViewToolbar } from './hooks/useViewToolbar';
import { useToast } from './hooks/useToast';
import { AssetCard } from './components/AssetCard/AssetCard';
import { ViewEditor } from './components/ViewEditor/ViewEditor';
import { ViewSubscriptionManager } from './components/ViewEditor/ViewSubscriptionManager';
import { ViewManager } from './components/ViewManager/ViewManager';
import { BulkDelete } from './components/BulkDelete/BulkDelete';
import { ProviderSettings } from './components/Settings/ProviderSettings';
import { SubscriptionManager } from './components/Settings/SubscriptionManager';
import { DataManager } from './components/Settings/DataManager';
import { ThemePicker } from './components/Settings/ThemePicker';
import { LanguagePicker } from './components/Settings/LanguagePicker';
import { ToastContainer } from './components/Toast/Toast';
import { DexPage } from './components/DexPage/DexPage';
import { t } from './lib/i18n';
import { useLocale } from './hooks/useLocale';
import './App.css';

type Tab = 'dashboard' | 'dex' | 'providers' | 'settings';
type ViewMode = 'grid' | 'list' | 'compact';

function App() {
  useLocale(); // 訂閱語言變更，觸發整個 App 重新渲染
  const [activeTab, setActiveTab] = useState<Tab>('dashboard');
  const [viewMode, setViewMode] = useState<ViewMode>(() => {
    const saved = localStorage.getItem('sb_view_mode');
    if (saved === 'list' || saved === 'compact') return saved;
    return 'grid';
  });
  const [showSubscriptionManager, setShowSubscriptionManager] = useState(false);
  const [showViewManager, setShowViewManager] = useState(false);
  const [showBulkDelete, setShowBulkDelete] = useState(false);
  const [showAddSubscription, setShowAddSubscription] = useState(false);
  const toast = useToast();

  const handleSetViewMode = (mode: ViewMode) => {
    setViewMode(mode);
    localStorage.setItem('sb_view_mode', mode);
  };
  const {
    views,
    activeViewId,
    activeViewSubscriptionIds,
    viewSubCounts,
    setActiveView,
    createView,
    renameView,
    deleteView,
    addSubscriptionToView,
    removeSubscriptionFromView,
    refresh: refreshViews,
  } = useViews('asset');

  const {
    editorState, setEditorState, pinnedViewIds, toolbarViews,
    handleCreateView, handleRequestRename, handleEditorConfirm,
    handleDeleteView, togglePinView,
  } = useViewToolbar({
    views, activeViewId, createView, renameView, deleteView, toast,
    storageKey: 'sb_pinned_views',
  });

  const {
    subscriptions,
    providerInfoList,
    loading,
    addSubscription,
    removeSubscription,
    removeSubscriptions,
    updateSubscription,
    getSelectedProvider,
    getAssetType,
    getRefreshInterval,
    refresh: refreshAssets,
  } = useAssetData();

  const handleAdd = useCallback(async (symbol: string, providerId?: string, assetType?: 'crypto' | 'stock') => {
    await addSubscription(symbol, undefined, providerId, assetType);
    await refreshViews();
  }, [addSubscription, refreshViews]);

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
    invoke('set_visible_subscriptions', { ids, scope: 'asset' }).catch(err =>
      console.error('Failed to set visible subscriptions:', err)
    );
  }, [viewFilteredSubs]);

  const handleCopySymbols = () => {
    const symbols = viewFilteredSubs.map(s => s.symbol).join(', ');
    navigator.clipboard.writeText(symbols).then(() => {
      toast.success(t.common.copied, t.subs.symbolsCopied(viewFilteredSubs.length));
    }).catch(() => {
      toast.error(t.common.copyFailed);
    });
  };

  return (
    <div className="app">
      <ToastContainer toasts={toast.toasts} onRemove={toast.removeToast} />

      <header className="app-header">
        <h1>StockenBoard</h1>
        <nav className="app-nav">
          <button className={`nav-btn ${activeTab === 'dashboard' ? 'active' : ''}`} onClick={() => setActiveTab('dashboard')}>{t.nav.dashboard}</button>
          <button className={`nav-btn ${activeTab === 'dex' ? 'active' : ''}`} onClick={() => setActiveTab('dex')}>{t.nav.dex}</button>
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
                <button className="btn-add" onClick={() => setShowAddSubscription(true)}>{t.subs.addSub}</button>
              </div>
            ) : (
              <>
                <div className="dashboard-toolbar">
                  <div className="dashboard-filters" role="tablist" aria-label={t.nav.pageSwitch}>
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
                        className={`manage-subs-btn ${showSubscriptionManager ? 'active' : ''}`}
                        onClick={() => setShowSubscriptionManager(prev => !prev)}
                      >
                        {t.subs.manageSubs}
                      </button>
                    )}
                    <button className="add-sub-btn" onClick={() => setShowAddSubscription(true)} title={t.subs.addSub}>
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="16"/><line x1="8" y1="12" x2="16" y2="12"/></svg>
                    </button>
                    <button className="copy-symbols-btn" onClick={handleCopySymbols} title={t.subs.copyAllSymbols}>
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
                  {viewFilteredSubs.map((sub) => (
                    <AssetCard
                      key={sub.id}
                      subscription={sub}
                      providers={providerInfoList}
                      currentProviderId={getSelectedProvider(sub.id)}
                      assetType={getAssetType(sub.id)}
                      refreshInterval={getRefreshInterval(sub.selected_provider_id)}
                      onRemove={handleRemove}
                      onEdit={updateSubscription}
                      viewMode={viewMode}
                      isCustomView={isCustomView}
                    />
                  ))}
                </div>
              </>
            )}
          </div>
        )}

        {activeTab === 'dex' && (
          <DexPage onToast={toast} />
        )}

        {activeTab === 'providers' && (
          <div className="providers-page">
            <ProviderSettings onSaved={() => toast.success(t.providers.settingsSaved)} />
          </div>
        )}

        {activeTab === 'settings' && (
          <div className="settings">
            <ThemePicker />
            <LanguagePicker />
            <DataManager
              views={views}
              onRefresh={() => { refreshAssets(); refreshViews(); }}
              onToast={(type, title, msg) => toast[type](title, msg)}
            />
          </div>
        )}
      </main>

      {editorState && (
        <ViewEditor
          mode={editorState.mode}
          currentName={editorState.mode === 'rename' ? editorState.currentName : undefined}
          existingNames={views.map(v => v.name)}
          onConfirm={handleEditorConfirm}
          onCancel={() => setEditorState(null)}
        />
      )}

      {showSubscriptionManager && activeViewSubscriptionIds !== null && (
        <ViewSubscriptionManager
          allSubscriptions={subscriptions}
          viewSubscriptionIds={activeViewSubscriptionIds}
          onToggleSubscription={(subId, add) => {
            if (add) addSubscriptionToView(activeViewId, subId);
            else removeSubscriptionFromView(activeViewId, subId);
          }}
          onClose={() => setShowSubscriptionManager(false)}
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

      {showBulkDelete && (
        <BulkDelete
          subscriptions={viewFilteredSubs}
          isCustomView={isCustomView}
          onConfirm={async (ids) => {
            if (ids.size === 0) return;
            const count = ids.size;
            if (isCustomView) {
              for (const id of ids) {
                await removeSubscriptionFromView(activeViewId, id);
              }
              setShowBulkDelete(false);
              toast.info(t.subs.bulkRemoveView, t.subs.bulkRemovedView(count));
            } else {
              await removeSubscriptions([...ids]);
              setShowBulkDelete(false);
              toast.info(t.subs.bulkUnsubscribe, t.subs.bulkUnsubscribed(count));
            }
          }}
          onClose={() => setShowBulkDelete(false)}
        />
      )}
      {showAddSubscription && (
        <div className="modal-backdrop sub-modal-backdrop" onClick={() => setShowAddSubscription(false)}>
          <div className="modal-container sub-modal" onClick={e => e.stopPropagation()}>
            <div className="sub-modal-header">
              <h4 className="sub-modal-title">{t.subs.addSub}</h4>
              <button className="vsm-close" onClick={() => setShowAddSubscription(false)}>✕</button>
            </div>
            <div className="sub-modal-body">
              <SubscriptionManager
                onBatchAdd={handleAdd}
                subscriptions={subscriptions}
                providers={providerInfoList}
                onToast={(type, title, msg) => toast[type](title, msg)}
              />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
