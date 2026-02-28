/**
 * 歷史表格元件 — 從 HistoryPage.tsx 抽出
 */
import { useCallback } from 'react';
import { formatNumber } from '../../lib/format';
import { t } from '../../lib/i18n';
import type { PriceHistoryRecord } from '../../types';

type SessionFilter = 'regular' | 'pre' | 'post';

function fmtTime(ts: number) { return new Date(ts * 1000).toLocaleString(); }

interface HistoryTableProps {
  records: PriceHistoryRecord[];
  session: SessionFilter;
  tzLabel: string;
}

export function HistoryTable({ records, session, tzLabel }: HistoryTableProps) {
  const getPrice = useCallback((r: PriceHistoryRecord) => {
    if (session === 'pre' && r.pre_price != null) return r.pre_price;
    if (session === 'post' && r.post_price != null) return r.post_price;
    return r.price;
  }, [session]);

  return (
    <div className="h-card history-table-container">
      <table className="history-table">
        <thead>
          <tr>
            <th>{t.history.time} ({tzLabel})</th>
            <th>{t.history.price}</th>
            <th>{t.history.changePct}</th>
            <th>{t.history.volume}</th>
            <th>{t.history.provider}</th>
          </tr>
        </thead>
        <tbody>
          {records.map(r => (
            <tr key={r.id}>
              <td>{fmtTime(r.recorded_at)}</td>
              <td>{formatNumber(getPrice(r))}</td>
              <td style={{ color: (r.change_pct ?? 0) >= 0 ? 'var(--up-color)' : 'var(--down-color)' }}>
                {r.change_pct != null ? `${r.change_pct >= 0 ? '+' : ''}${r.change_pct.toFixed(2)}%` : '-'}
              </td>
              <td>{r.volume != null ? formatNumber(r.volume) : '-'}</td>
              <td>{r.provider_id}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
