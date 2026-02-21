import { useState } from 'react';
import type { Subscription } from '../../types';
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

  const filtered = allSubscriptions.filter((sub) => {
    if (!search.trim()) return true;
    const q = search.trim().toLowerCase();
    return (
      sub.symbol.toLowerCase().includes(q) ||
      (sub.display_name && sub.display_name.toLowerCase().includes(q))
    );
  });

  return (
    <div className="vsm-backdrop" onClick={onClose}>
      <div className="vsm-modal" role="dialog" aria-modal="true" aria-label="管理訂閱" onClick={(e) => e.stopPropagation()}>
        <div className="vsm-header">
          <h2 className="vsm-title">管理訂閱</h2>
          <button className="vsm-close" onClick={onClose} aria-label="關閉">✕</button>
        </div>
        <input
          className="vsm-search"
          type="text"
          placeholder="搜尋代號或名稱..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          autoFocus
        />
        {filtered.length === 0 ? (
          <p className="vsm-empty">{allSubscriptions.length === 0 ? '目前沒有任何訂閱' : '找不到符合的訂閱'}</p>
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
                      aria-label={`${checked ? '移除' : '加入'} ${sub.symbol}`}
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
          <span className="vsm-count">已選 {viewSubscriptionIds.length} / {allSubscriptions.length}</span>
        </div>
      </div>
    </div>
  );
}
