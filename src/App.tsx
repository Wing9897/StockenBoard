import { useState } from 'react';
import { useAssetData } from './hooks/useAssetData';
import { useViews } from './hooks/useViews';
import { useToast } from './hooks/useToast';
import { AssetCard } from './components/AssetCard/AssetCard';
import { ViewEditor } from './components/ViewEditor/ViewEditor';
import { ViewSubscriptionManager } from './components/ViewEditor/ViewSubscriptionManager';
import { ProviderSettings } from './components/Settings/ProviderSettings';
import { SubscriptionManager } from './components/Settings/SubscriptionManager';
import { DataManager } from './components/Settings/DataManager';
import { ToastContainer } from './components/Toast/Toast';
import './App.css';

type Tab = 'dashboard' | 'settings';
type ViewMode = 'grid' | 'list';
type EditorState = null | { mode: 'create' } | { mode: 'rename'; viewId: number; currentName: string };

function App() {
  const [activeTab, setActiveTab] = useState<Tab>('dashboard');
  const [viewMode, setViewMode] = useState<ViewMode>(() => {
    const saved = localStorage.getItem('sb_view_mode');
    return saved === 'list' ? 'list' : 'grid';
  });
  const [editorState, setEditorState] = useState<EditorState>(null);
  const [showSubscriptionManager, setShowSubscriptionManager] = useState(false);
  const [showViewManager, setShowViewManager] = useState(false);
  const [showBulkDelete, setShowBulkDelete] = useState(false);
  const [bulkDeleteIds, setBulkDeleteIds] = useState<Set<number>>(new Set());
  const [pinnedViewIds, setPinnedViewIds] = useState<number[]>(() => {
    try { return JSON.parse(localStorage.getItem('sb_pinned_views') || '[]'); } catch { return []; }
  });
  const toast = useToast();

  // 持久化 viewMode
  const handleSetViewMode = (mode: ViewMode) => {
    setViewMode(mode);
    localStorage.setItem('sb_view_mode', mode);
  };
  const {
    views,
    activeViewId,
    activeViewSubscriptionIds,
    setActiveView,
    createView,
    renameView,
    deleteView,
    addSubscriptionToView,
    removeSubscriptionFromView,
    refresh: refreshViews,
  } = useViews();
  const {
    subscriptions,
    providerInfoList,
    loading,
    addSubscription,
    removeSubscription,
    updateSubscription,
    getAsset,
    getError,
    getSelectedProvider,
    getAssetType,
    getRefreshTiming,
    refresh: refreshAssets,
  } = useAssetData();

  const handleAdd = async (symbol: string, defaultProviderId?: string, assetType?: 'crypto' | 'stock') => {
    await addSubscription(symbol, undefined, defaultProviderId, assetType);
  };

  const handleRemove = async (id: number) => {
    const sub = subscriptions.find(s => s.id === id);
    await removeSubscription(id);
    toast.info('已移除', sub ? `${sub.symbol} 已取消訂閱` : '已取消訂閱');
  };


  // Filter subscriptions by active view
  const viewFilteredSubs = activeViewSubscriptionIds === null
    ? subscriptions
    : subscriptions.filter(sub => activeViewSubscriptionIds.includes(sub.id));

  const handleCreateView = () => setEditorState({ mode: 'create' });

  const handleRequestRename = (viewId: number) => {
    const view = views.find(v => v.id === viewId);
    if (view) setEditorState({ mode: 'rename', viewId, currentName: view.name });
  };

  const handleEditorConfirm = (name: string) => {
    if (!editorState) return;
    if (editorState.mode === 'create') {
      createView(name)
        .then(() => toast.success('已建立', `頁面「${name}」已建立`))
        .catch(err => toast.error('建立頁面失敗', err instanceof Error ? err.message : String(err)));
    } else {
      renameView(editorState.viewId, name)
        .then(() => toast.success('已重新命名', `頁面已更名為「${name}」`))
        .catch(err => toast.error('重新命名失敗', err instanceof Error ? err.message : String(err)));
    }
    setEditorState(null);
  };

  const handleDeleteView = (viewId: number) => {
    if (confirm('確定要刪除此頁面嗎？')) {
      const viewName = views.find(v => v.id === viewId)?.name;
      deleteView(viewId)
        .then(() => toast.success('已刪除', viewName ? `頁面「${viewName}」已刪除` : '頁面已刪除'))
        .catch(err => toast.error('刪除頁面失敗', err instanceof Error ? err.message : String(err)));
      setPinnedViewIds(prev => {
        const next = prev.filter(id => id !== viewId);
        localStorage.setItem('sb_pinned_views', JSON.stringify(next));
        return next;
      });
    }
  };

  const togglePinView = (viewId: number) => {
    setPinnedViewIds(prev => {
      const next = prev.includes(viewId) ? prev.filter(id => id !== viewId) : [...prev, viewId];
      localStorage.setItem('sb_pinned_views', JSON.stringify(next));
      return next;
    });
  };

  const openBulkDelete = () => {
    setBulkDeleteIds(new Set());
    setShowBulkDelete(true);
  };

  const toggleBulkDeleteId = (id: number) => {
    setBulkDeleteIds(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleBulkDelete = async () => {
    if (bulkDeleteIds.size === 0) return;
    const count = bulkDeleteIds.size;
    for (const id of bulkDeleteIds) {
      await removeSubscription(id);
    }
    setShowBulkDelete(false);
    toast.info('批量移除', `已取消 ${count} 個訂閱`);
  };

  // Views shown in toolbar: default + pinned + active (if not already shown)
  const sortedViews = [...views].sort((a, b) => {
    if (a.is_default) return -1;
    if (b.is_default) return 1;
    return a.sort_order - b.sort_order;
  });

  const toolbarViews = (() => {
    const pinned = sortedViews.filter(v =>
      v.is_default || pinnedViewIds.includes(v.id) || v.id === activeViewId
    );
    // 沒有置頂時，自動顯示前 5 個頁面
    const MAX_AUTO = 5;
    if (pinned.length <= 1 && sortedViews.length > 1) {
      const auto = sortedViews.slice(0, MAX_AUTO);
      if (activeViewId && !auto.find(v => v.id === activeViewId)) {
        const activeView = sortedViews.find(v => v.id === activeViewId);
        if (activeView) auto.push(activeView);
      }
      return auto;
    }
    return pinned;
  })();

  return (
    <div className="app">
      <ToastContainer toasts={toast.toasts} onRemove={toast.removeToast} />

      <header className="app-header">
        <h1>StockenBoard</h1>
        <nav className="app-nav">
          <button className={`nav-btn ${activeTab === 'dashboard' ? 'active' : ''}`} onClick={() => setActiveTab('dashboard')}>主頁</button>
          <button className={`nav-btn ${activeTab === 'settings' ? 'active' : ''}`} onClick={() => setActiveTab('settings')}>設定</button>
        </nav>
      </header>

      <main className="app-main">
        {activeTab === 'dashboard' && (
          <div className="dashboard">
            {loading ? (
              <div className="loading">載入中...</div>
            ) : subscriptions.length === 0 ? (
              <div className="empty-state">
                <p>尚未訂閱任何資產</p>
                <button className="btn-add" onClick={() => setActiveTab('settings')}>前往設定新增</button>
              </div>
            ) : (
              <>
                <div className="dashboard-toolbar">
                  <div className="dashboard-filters" role="tablist" aria-label="頁面切換">
                    {toolbarViews.map(view => (
                      <button
                        key={view.id}
                        className={`view-tag ${view.id === activeViewId ? 'active' : ''} ${view.is_default ? 'default' : ''}`}
                        role="tab"
                        aria-selected={view.id === activeViewId}
                        onClick={() => setActiveView(view.id)}
                      >
                        {view.name}
                        {view.is_default && ` (${viewFilteredSubs.length})`}
                      </button>
                    ))}
                    {views.filter(v => !v.is_default).length > 0 && (
                      <button className="view-manager-btn" onClick={() => setShowViewManager(true)} title="管理頁面">⋯</button>
                    )}
                  </div>
                  <div className="toolbar-right">
                    {activeViewSubscriptionIds !== null && (
                      <button
                        className={`manage-subs-btn ${showSubscriptionManager ? 'active' : ''}`}
                        onClick={() => setShowSubscriptionManager(prev => !prev)}
                      >
                        管理訂閱
                      </button>
                    )}
                    <button className="add-view-btn" onClick={handleCreateView} title="新增頁面">+</button>
                    <div className="view-toggle">
                      <button className={`view-btn ${viewMode === 'grid' ? 'active' : ''}`} onClick={() => handleSetViewMode('grid')} title="方塊顯示">▦</button>
                      <button className={`view-btn ${viewMode === 'list' ? 'active' : ''}`} onClick={() => handleSetViewMode('list')} title="列表顯示">☰</button>
                    </div>
                    <button className="bulk-delete-btn" onClick={openBulkDelete} title="批量取消訂閱">🗑</button>
                  </div>
                </div>
                <div className={viewMode === 'grid' ? 'asset-grid' : 'asset-list'}>
                  {viewFilteredSubs.map((sub) => (
                    <AssetCard
                      key={sub.id}
                      asset={getAsset(sub.id, sub.symbol)}
                      error={getError(sub.id, sub.symbol)}
                      subscription={sub}
                      providers={providerInfoList}
                      currentProviderId={getSelectedProvider(sub.id)}
                      assetType={getAssetType(sub.id)}
                      refreshTiming={getRefreshTiming(sub.id)}
                      onRemove={handleRemove}
                      onEdit={updateSubscription}
                      viewMode={viewMode}
                    />
                  ))}
                </div>
              </>
            )}
          </div>
        )}

        {activeTab === 'settings' && (
          <div className="settings">
            <SubscriptionManager onBatchAdd={handleAdd} subscriptions={subscriptions} onToast={(title, msg) => toast.success(title, msg)} />
            <DataManager
              subscriptions={subscriptions}
              views={views}
              onRefresh={() => { refreshAssets(); refreshViews(); }}
              onToast={(type, title, msg) => toast[type](title, msg)}
            />
            <ProviderSettings onSaved={() => toast.success('設定已儲存')} />
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
        <div className="vm-backdrop" onClick={() => setShowViewManager(false)}>
          <div className="vm-modal" onClick={e => e.stopPropagation()}>
            <div className="vm-header">
              <h4 className="vm-title">管理頁面</h4>
              <button className="vsm-close" onClick={() => setShowViewManager(false)}>✕</button>
            </div>
            <ul className="vm-list">
              {sortedViews.filter(v => !v.is_default).map(view => (
                <li key={view.id} className={`vm-item ${view.id === activeViewId ? 'active' : ''}`}>
                  <button className="vm-item-name" onClick={() => { setActiveView(view.id); setShowViewManager(false); }}>
                    {view.name}
                  </button>
                  <div className="vm-item-actions">
                    <button
                      className={`vm-pin-btn ${pinnedViewIds.includes(view.id) ? 'pinned' : ''}`}
                      onClick={() => togglePinView(view.id)}
                      title={pinnedViewIds.includes(view.id) ? '取消置頂' : '置頂'}
                    >
                      {pinnedViewIds.includes(view.id) ? '★' : '☆'}
                    </button>
                    <button className="vm-action-btn" onClick={() => { handleRequestRename(view.id); setShowViewManager(false); }} title="重新命名">✎</button>
                    <button className="vm-action-btn danger" onClick={() => { handleDeleteView(view.id); }} title="刪除">✕</button>
                  </div>
                </li>
              ))}
            </ul>
            <div className="vm-footer">
              <button className="vm-add-btn" onClick={() => { handleCreateView(); setShowViewManager(false); }}>+ 新增頁面</button>
            </div>
          </div>
        </div>
      )}

      {showBulkDelete && (
        <div className="bd-backdrop" onClick={() => setShowBulkDelete(false)}>
          <div className="bd-modal" onClick={e => e.stopPropagation()}>
            <div className="bd-header">
              <h4 className="bd-title">批量取消訂閱</h4>
              <button className="vsm-close" onClick={() => setShowBulkDelete(false)}>✕</button>
            </div>
            <div className="bd-actions">
              <button className="dm-pick-btn" onClick={() => setBulkDeleteIds(new Set(viewFilteredSubs.map(s => s.id)))}>全選</button>
              <button className="dm-pick-btn" onClick={() => setBulkDeleteIds(new Set())}>取消全選</button>
            </div>
            <ul className="bd-list">
              {viewFilteredSubs.map(sub => (
                <li key={sub.id} className="bd-item">
                  <label className="bd-label">
                    <input
                      type="checkbox"
                      checked={bulkDeleteIds.has(sub.id)}
                      onChange={() => toggleBulkDeleteId(sub.id)}
                    />
                    <span className="bd-symbol">{sub.symbol}</span>
                    {sub.display_name && <span className="bd-display-name">{sub.display_name}</span>}
                    <span className={`bd-type ${sub.asset_type || 'crypto'}`}>{sub.asset_type === 'stock' ? '股' : '幣'}</span>
                  </label>
                </li>
              ))}
            </ul>
            <div className="bd-footer">
              <span className="bd-count">{bulkDeleteIds.size} / {viewFilteredSubs.length} 已選</span>
              <button className="bd-confirm" onClick={handleBulkDelete} disabled={bulkDeleteIds.size === 0}>
                移除 ({bulkDeleteIds.size})
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
