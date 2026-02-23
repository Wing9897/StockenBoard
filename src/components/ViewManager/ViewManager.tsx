import { View } from '../../types';

interface ViewManagerProps {
  views: View[];
  activeViewId: number;
  pinnedViewIds: number[];
  onSelectView: (viewId: number) => void;
  onTogglePin: (viewId: number) => void;
  onRename: (viewId: number) => void;
  onDelete: (viewId: number) => void;
  onCreate: () => void;
  onClose: () => void;
}

export function ViewManager({
  views, activeViewId, pinnedViewIds,
  onSelectView, onTogglePin, onRename, onDelete, onCreate, onClose,
}: ViewManagerProps) {
  const sorted = [...views]
    .filter(v => !v.is_default)
    .sort((a, b) => a.id - b.id);

  return (
    <div className="vm-backdrop" onClick={onClose}>
      <div className="vm-modal" onClick={e => e.stopPropagation()}>
        <div className="vm-header">
          <h4 className="vm-title">管理頁面</h4>
          <button className="vsm-close" onClick={onClose}>✕</button>
        </div>
        <ul className="vm-list">
          {sorted.map(view => (
            <li key={view.id} className={`vm-item ${view.id === activeViewId ? 'active' : ''}`}>
              <button className="vm-item-name" onClick={() => { onSelectView(view.id); onClose(); }}>
                {view.name}
              </button>
              <div className="vm-item-actions">
                <button
                  className={`vm-pin-btn ${pinnedViewIds.includes(view.id) ? 'pinned' : ''}`}
                  onClick={() => onTogglePin(view.id)}
                  title={pinnedViewIds.includes(view.id) ? '取消置頂' : '置頂'}
                >
                  {pinnedViewIds.includes(view.id) ? '★' : '☆'}
                </button>
                <button className="vm-action-btn" onClick={() => { onRename(view.id); onClose(); }} title="重新命名">✎</button>
                <button className="vm-action-btn danger" onClick={() => onDelete(view.id)} title="刪除">✕</button>
              </div>
            </li>
          ))}
        </ul>
        <div className="vm-footer">
          <button className="vm-add-btn" onClick={() => { onCreate(); onClose(); }}>+ 新增頁面</button>
        </div>
      </div>
    </div>
  );
}
