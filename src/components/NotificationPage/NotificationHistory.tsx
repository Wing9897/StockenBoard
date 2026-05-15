import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface HistoryRow {
  id: number;
  rule_id: number;
  channel_id: number;
  status: string;
  price: number;
  message: string;
  error: string | null;
  sent_at: number;
}

export function NotificationHistory() {
  const [history, setHistory] = useState<HistoryRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [fromDate, setFromDate] = useState('');
  const [toDate, setToDate] = useState('');

  const fetchHistory = useCallback(async (from?: number, to?: number) => {
    setLoading(true);
    try {
      const data = await invoke<HistoryRow[]>('get_notification_history', {
        ruleId: null,
        from: from ?? null,
        to: to ?? null,
        limit: 50,
      });
      setHistory(data);
    } catch (e) {
      console.error('Failed to fetch history:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { fetchHistory(); }, [fetchHistory]);

  const handleFilter = () => {
    const from = fromDate ? Math.floor(new Date(fromDate).getTime() / 1000) : undefined;
    const to = toDate ? Math.floor(new Date(toDate + 'T23:59:59').getTime() / 1000) : undefined;
    fetchHistory(from, to);
  };

  const formatTime = (ts: number) => {
    return new Date(ts * 1000).toLocaleString();
  };

  if (loading) return <div className="notification-placeholder"><p>載入中...</p></div>;

  return (
    <div className="notification-history">
      <div className="history-filter">
        <label className="filter-field">
          <span>從</span>
          <input type="date" value={fromDate} onChange={e => setFromDate(e.target.value)} />
        </label>
        <label className="filter-field">
          <span>至</span>
          <input type="date" value={toDate} onChange={e => setToDate(e.target.value)} />
        </label>
        <button className="btn-filter" onClick={handleFilter}>篩選</button>
      </div>

      {history.length === 0 ? (
        <div className="notification-placeholder"><p>尚無通知紀錄</p></div>
      ) : (
        <div className="history-items">
          {history.map(item => (
            <div key={item.id} className="history-item">
              <div className="history-meta">
                <span className={`history-status ${item.status}`}>
                  {item.status === 'success' ? '✓ 成功' : '✗ 失敗'}
                </span>
                <span className="history-time">{formatTime(item.sent_at)}</span>
                <span className="history-price">${item.price.toLocaleString()}</span>
              </div>
              {item.status === 'failed' && item.error && (
                <div className="history-error">{item.error}</div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
