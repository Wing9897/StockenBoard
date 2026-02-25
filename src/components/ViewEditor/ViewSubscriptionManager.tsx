import { useState } from 'react';
import type { Subscription } from '../../types';
import { t } from '../../lib/i18n';
import './ViewEditor.css';

interface ViewSubscriptionManagerProps {
  allSubscriptions: Subscription[];
  viewSubscriptionIds: number[];
  onToggleSubscription: (subscriptionId: number, add: boolean) => void;
  onClose: () => void;
}

export function ViewSubscriptionManager({
  allSubscriptions,
  viewSubscriptionIds,
  onToggleSubscription,
  onClose,
}: ViewSubscriptionManagerProps) {
  const [search, setSearch] = useState('');

  const searchTerms = search
    .split(/[,\s\n]+/)
    .map(s => s.trim().toLowerCase())
    .filter(Boolean);

  const filtered = allSubscriptions.filter((sub) => {
    if (searchTerms.length === 0) return true;
    return searchTerms.some(q =>
      sub.symbol.toLowerCase().includes(q) ||
      (sub.display_name && sub.display_name.toLowerCase().includes(q))
    );
  });

  const filteredAllChecked = filtered.length > 0 && filtered.every(s => viewSubscriptionIds.includes(s.id));
  const filteredNoneChecked = filtered.every(s => !viewSubscriptionIds.includes(s.id));

  const handleSelectAllFiltered = () => {
    for (const sub of filtered) {
      if (!viewSubscriptionIds.includes(sub.id)) {
        onToggleSubscription(sub.id, true);
      }
    }
  };

  const handleClearFiltered = () => {
    for (const sub of filtered) {
      if (viewSubscriptionIds.includes(sub.id)) {
        onToggleSubscription(sub.id, false);
      }
    }
  };

  return (
    <div className="modal-backdrop vsm-backdrop" onClick={onClose}>
      <div className="modal-container vsm-modal" role="dialog" aria-modal="true" aria-label={t.subs.manageSubs} onClick={(e) => e.stopPropagation()}>
        <div className="vsm-header">
          <h2 className="vsm-title">{t.subs.manageSubs}</h2>
          <button className="vsm-close" onClick={onClose} aria-label={t.common.close}>✕</button>
        </div>
        <input
          className="vsm-search"
          type="text"
          placeholder={t.subs.searchPlaceholder}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          autoFocus
        />
        <div className="vsm-bulk-actions">
          <button className="vsm-bulk-btn" onClick={handleSelectAllFiltered} disabled={filteredAllChecked || filtered.length === 0}>
            {t.subs.selectAll} ({filtered.length})
          </button>
          <button className="vsm-bulk-btn" onClick={handleClearFiltered} disabled={filteredNoneChecked || filtered.length === 0}>
            {t.subs.clearAll}
          </button>
        </div>
        {filtered.length === 0 ? (
          <p className="vsm-empty">{allSubscriptions.length === 0 ? t.subs.noSubsYet : t.subs.noMatch}</p>
        ) : (
          <ul className="vsm-list" role="list">
            {filtered.map((sub) => {
              const checked = viewSubscriptionIds.includes(sub.id);
              return (
                <li key={sub.id} className="vsm-item">
                  <label className="vsm-label">
                    <input
                      type="checkbox"
                      className="vsm-checkbox"
                      checked={checked}
                      onChange={() => onToggleSubscription(sub.id, !checked)}
                      aria-label={`${checked ? t.common.remove : t.common.add} ${sub.symbol}`}
                    />
                    <span className="vsm-symbol">{sub.symbol}</span>
                    {sub.display_name && <span className="vsm-display-name">{sub.display_name}</span>}
                  </label>
                </li>
              );
            })}
          </ul>
        )}
        <div className="vsm-footer">
          <span className="vsm-count">{t.subs.selected(viewSubscriptionIds.length, allSubscriptions.length)}</span>
        </div>
      </div>
    </div>
  );
}
