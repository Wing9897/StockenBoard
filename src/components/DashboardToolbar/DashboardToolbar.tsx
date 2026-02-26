import { View, ViewMode } from '../../types';
import { IconAdd, IconCopy, IconGear, IconTrash } from '../Icons';
import { t } from '../../lib/i18n';

interface DashboardToolbarProps {
  /** toolbar 上顯示的 view 列表（含 pinned + default） */
  toolbarViews: View[];
  /** 所有 view（用來判斷是否顯示 ⋯ 按鈕） */
  views: View[];
  activeViewId: number;
  /** 全部訂閱數（顯示在 default view tag 上） */
  totalCount: number;
  /** 各 view 的訂閱數 */
  viewSubCounts: Record<number, number>;
  /** 是否為自訂 view */
  isCustomView: boolean;
  /** 是否正在顯示訂閱管理面板 */
  showSubManager: boolean;

  viewMode: ViewMode;
  onSetViewMode: (mode: ViewMode) => void;
  onSelectView: (id: number) => void;
  onCreateView: () => void;
  onOpenViewManager: () => void;
  onToggleSubManager: () => void;
  onAdd: () => void;
  onCopy: () => void;
  onBatchActions: () => void;
  onBulkDelete: () => void;

  /** i18n 文字 */
  addTitle: string;
  copyTitle: string;
  bulkDeleteTitle: string;
  /** tablist aria-label */
  tabListLabel: string;
}

export function DashboardToolbar({
  toolbarViews, views, activeViewId, totalCount, viewSubCounts,
  isCustomView, showSubManager, viewMode,
  onSetViewMode, onSelectView, onCreateView, onOpenViewManager,
  onToggleSubManager, onAdd, onCopy, onBatchActions, onBulkDelete,
  addTitle, copyTitle, bulkDeleteTitle, tabListLabel,
}: DashboardToolbarProps) {
  const hasCustomViews = views.some(v => !v.is_default);

  return (
    <div className="dashboard-toolbar">
      <div className="dashboard-filters" role="tablist" aria-label={tabListLabel}>
        {toolbarViews.map(view => (
          <button
            key={view.id}
            className={`view-tag ${view.id === activeViewId ? 'active' : ''} ${view.is_default ? 'default' : ''}`}
            role="tab"
            aria-selected={view.id === activeViewId}
            onClick={() => onSelectView(view.id)}
          >
            {view.is_default ? t.providers.all : view.name}
            {view.is_default
              ? ` (${totalCount})`
              : ` (${viewSubCounts[view.id] ?? 0})`
            }
          </button>
        ))}
        {hasCustomViews && (
          <button className="view-manager-btn" onClick={onOpenViewManager} title={t.views.manageViews}>⋯</button>
        )}
        <button className="add-view-btn" onClick={onCreateView} title={t.views.addView}>+</button>
      </div>
      <div className="toolbar-right">
        {isCustomView && (
          <button
            className={`manage-subs-btn ${showSubManager ? 'active' : ''}`}
            onClick={onToggleSubManager}
          >
            {t.subs.manageSubs}
          </button>
        )}
        <button className="add-sub-btn" onClick={onAdd} title={addTitle}>
          <IconAdd />
        </button>
        <button className="copy-symbols-btn" onClick={onCopy} title={copyTitle}>
          <IconCopy />
        </button>
        <div className="view-toggle">
          <button className={`view-btn ${viewMode === 'compact' ? 'active' : ''}`} onClick={() => onSetViewMode('compact')} title={t.viewMode.compact}>▪</button>
          <button className={`view-btn ${viewMode === 'grid' ? 'active' : ''}`} onClick={() => onSetViewMode('grid')} title={t.viewMode.grid}>▦</button>
          <button className={`view-btn ${viewMode === 'list' ? 'active' : ''}`} onClick={() => onSetViewMode('list')} title={t.viewMode.list}>☰</button>
        </div>
        <button className="batch-actions-btn" onClick={onBatchActions} title={t.dashboard.batchActions}>
          <IconGear />
        </button>
        <button className="bulk-delete-btn" onClick={onBulkDelete} title={bulkDeleteTitle}>
          <IconTrash />
        </button>
      </div>
    </div>
  );
}
