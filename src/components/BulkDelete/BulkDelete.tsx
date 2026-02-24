import { useState } from 'react';
import { Subscription } from '../../types';

interface BulkDeleteProps {
  subscriptions: Subscription[];
  isCustomView: boolean;
  onConfirm: (ids: Set<number>) => void;
  onClose: () => void;
  /** 自訂每行顯示的主文字，預設用 symbol */
  getLabel?: (sub: Subscription) => { primary: string; secondary?: string };
}

export function BulkDelete({ subscriptions, isCustomView, onConfirm, onClose, getLabel }: BulkDeleteProps) {
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());

  const toggle = (id: number) => {
    setSelectedIds(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const defaultLabel = (sub: Subscription) => ({
    primary: sub.display_name || sub.symbol,
    secondary: sub.display_name ? sub.symbol : undefined,
  });

  const labelFn = getLabel || defaultLabel;

  return (
    <div className="bd-backdrop" onClick={onClose}>
      <div className="bd-modal" onClick={e => e.stopPropagation()}>
        <div className="bd-header">
          <h4 className="bd-title">{isCustomView ? '批量移除顯示' : '批量取消訂閱'}</h4>
          <button className="vsm-close" onClick={onClose}>✕</button>
        </div>
        <div className="bd-actions">
          <button className="dm-pick-btn" onClick={() => setSelectedIds(new Set(subscriptions.map(s => s.id)))}>全選</button>
          <button className="dm-pick-btn" onClick={() => setSelectedIds(new Set())}>取消全選</button>
        </div>
        <ul className="bd-list">
          {subscriptions.map(sub => {
            const label = labelFn(sub);
            return (
              <li key={sub.id} className="bd-item">
                <label className="bd-label">
                  <input type="checkbox" checked={selectedIds.has(sub.id)} onChange={() => toggle(sub.id)} />
                  <span className="bd-symbol">{label.primary}</span>
                  {label.secondary && <span className="bd-display-name">{label.secondary}</span>}
                  <span className={`bd-type ${sub.asset_type}`}>{sub.asset_type === 'stock' ? '股' : '幣'}</span>
                </label>
              </li>
            );
          })}
        </ul>
        <div className="bd-footer">
          <span className="bd-count">{selectedIds.size} / {subscriptions.length} 已選</span>
          <button className="bd-confirm" onClick={() => onConfirm(selectedIds)} disabled={selectedIds.size === 0}>
            {isCustomView ? '移除顯示' : '取消訂閱'} ({selectedIds.size})
          </button>
        </div>
      </div>
    </div>
  );
}
