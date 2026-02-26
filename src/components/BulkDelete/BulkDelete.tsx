import { useState } from 'react';
import { Subscription } from '../../types';
import { t } from '../../lib/i18n';
import { useEscapeKey } from '../../hooks/useEscapeKey';

interface BulkDeleteProps {
  subscriptions: Subscription[];
  isCustomView: boolean;
  onConfirm: (ids: Set<number>) => void;
  onClose: () => void;
  getLabel?: (sub: Subscription) => { primary: string; secondary?: string };
}

export function BulkDelete({ subscriptions, isCustomView, onConfirm, onClose, getLabel }: BulkDeleteProps) {
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  useEscapeKey(onClose);

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
    <div className="modal-backdrop bd-backdrop" onClick={onClose}>
      <div className="modal-container bd-modal" role="dialog" aria-modal="true" aria-label={isCustomView ? t.subs.bulkRemoveView : t.subs.bulkUnsubscribe} onClick={e => e.stopPropagation()}>
        <div className="bd-header">
          <h4 className="bd-title">{isCustomView ? t.subs.bulkRemoveView : t.subs.bulkUnsubscribe}</h4>
          <button className="vsm-close" onClick={onClose} aria-label={t.common.close}>✕</button>
        </div>
        <div className="bd-actions">
          <button className="dm-pick-btn" onClick={() => setSelectedIds(new Set(subscriptions.map(s => s.id)))}>{t.subs.selectAll}</button>
          <button className="dm-pick-btn" onClick={() => setSelectedIds(new Set())}>{t.subs.clearAll}</button>
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
                  <span className={`bd-type ${sub.asset_type}`}>{sub.asset_type === 'stock' ? t.subForm.stockShort : t.subForm.cryptoShort}</span>
                </label>
              </li>
            );
          })}
        </ul>
        <div className="bd-footer">
          <span className="bd-count">{selectedIds.size} / {subscriptions.length} {t.common.selected}</span>
          <button className="bd-confirm" onClick={() => onConfirm(selectedIds)} disabled={selectedIds.size === 0}>
            {isCustomView ? t.subs.removeDisplay : t.subs.bulkUnsubscribe} ({selectedIds.size})
          </button>
        </div>
      </div>
    </div>
  );
}
