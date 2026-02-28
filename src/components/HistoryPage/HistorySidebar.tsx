/**
 * 歷史頁面側邊欄 — 從 HistoryPage.tsx 抽出
 */
import { useMemo } from 'react';
import { AssetIcon } from '../AssetCard/AssetIcon';
import { parsePairFromName } from '../../lib/format';
import { t } from '../../lib/i18n';
import type { Subscription } from '../../types';

type SubFilter = 'all' | 'asset' | 'dex';

interface HistorySidebarProps {
  subs: Subscription[];
  selectedId: number | null;
  filter: SubFilter;
  search: string;
  onSelectId: (id: number) => void;
  onSetFilter: (f: SubFilter) => void;
  onSetSearch: (s: string) => void;
  onToggle: (id: number, on: boolean) => void;
  onBatchToggle: (on: boolean) => void;
  onCollapse: () => void;
  onSaveRecordHours: (id: number, from: number | null, to: number | null) => void;
  tzLabel: string;
}

const noop = () => {};

export function HistorySidebar({
  subs, selectedId, filter, search,
  onSelectId, onSetFilter, onSetSearch,
  onToggle, onBatchToggle, onCollapse,
  onSaveRecordHours, tzLabel,
}: HistorySidebarProps) {
  const filtered = useMemo(() => {
    let list = subs;
    if (filter === 'dex') list = list.filter(s => s.sub_type === 'dex');
    else if (filter === 'asset') list = list.filter(s => s.sub_type === 'asset');
    const kw = search.split(/[,，;；]/).map(k => k.trim().toLowerCase()).filter(Boolean);
    if (kw.length) {
      list = list.filter(s => {
        const hay = `${s.display_name || ''} ${s.symbol} ${s.selected_provider_id}`.toLowerCase();
        return kw.some(q => hay.includes(q));
      });
    }
    return list;
  }, [subs, filter, search]);

  const recCount = useMemo(() => subs.filter(s => s.record_enabled).length, [subs]);
  const filtRecCount = useMemo(() => filtered.filter(s => s.record_enabled).length, [filtered]);
  const allOn = filtered.length > 0 && filtRecCount === filtered.length;

  return (
    <div className="h-card history-sidebar">
      <div className="history-sidebar-header">
        <span className="history-sidebar-title">{t.history.title}</span>
        {recCount > 0 && <span className="history-recording-badge">● {recCount}</span>}
        <button className="history-collapse-btn" onClick={onCollapse} title="收起">◀</button>
      </div>

      <div className="hseg equal">
        <button className={filter === 'all' ? 'active' : ''} onClick={() => onSetFilter('all')}>All</button>
        <button className={filter === 'asset' ? 'active' : ''} onClick={() => onSetFilter('asset')}>{t.history.spot}</button>
        <button className={filter === 'dex' ? 'active' : ''} onClick={() => onSetFilter('dex')}>{t.history.dex}</button>
      </div>

      <input className="history-search" type="text" placeholder={t.subs.searchPlaceholder} value={search} onChange={e => onSetSearch(e.target.value)} />

      <div className="history-batch-row">
        <span className="history-batch-count">{filtRecCount}/{filtered.length}</span>
        <button className="history-batch-btn enable" onClick={() => onBatchToggle(true)} disabled={allOn}>{t.subs.selectAll}</button>
        <button className="history-batch-btn disable" onClick={() => onBatchToggle(false)} disabled={filtRecCount === 0}>{t.subs.clearAll}</button>
      </div>

      <div className="history-sub-list">
        {filtered.map(s => {
          const isDex = s.sub_type === 'dex';
          const [from, to] = isDex ? parsePairFromName(s.display_name || s.symbol) : ['', ''];
          const isSelected = selectedId === s.id;
          const hasCustomHours = s.record_from_hour != null && s.record_to_hour != null;
          return (
            <div key={s.id}>
              <div className={`history-sub-item ${isSelected ? 'selected' : ''}`} onClick={() => onSelectId(s.id)}>
                {isDex ? (
                  <div className="history-dex-icons">
                    {from ? <AssetIcon symbol={from} className="asset-icon history-icon" onClick={noop} /> : <div className="asset-icon history-icon"><span className="asset-icon-fallback">?</span></div>}
                    {to ? <AssetIcon symbol={to} className="asset-icon history-icon" onClick={noop} /> : <div className="asset-icon history-icon"><span className="asset-icon-fallback">?</span></div>}
                  </div>
                ) : (
                  <AssetIcon symbol={s.symbol} className="asset-icon history-icon" onClick={noop} />
                )}
                <div className="history-sub-info">
                  <span className="history-sub-symbol">{s.display_name || s.symbol}</span>
                  <span className="history-sub-meta">
                    {s.selected_provider_id}
                    {s.record_enabled ? <span className="history-rec-dot">●</span> : null}
                    {hasCustomHours && <span className="history-hours-badge">{s.record_from_hour}–{s.record_to_hour}h</span>}
                  </span>
                </div>
                <button
                  className={`history-record-toggle ${s.record_enabled ? 'recording' : ''}`}
                  title={s.record_enabled ? t.history.disableRecord : t.history.enableRecord}
                  onClick={e => { e.stopPropagation(); onToggle(s.id, !s.record_enabled); }}
                >{s.record_enabled ? '●' : ''}</button>
              </div>
              {isSelected && s.record_enabled ? (
                <div className="history-hours-editor">
                  <div className="history-hours-row">
                    <span className="history-hours-label">{t.history.recordHours}</span>
                    <select className="history-hours-select"
                      value={hasCustomHours ? 'custom' : 'all'}
                      onChange={e => {
                        if (e.target.value === 'all') onSaveRecordHours(s.id, null, null);
                        else onSaveRecordHours(s.id, 16, 9);
                      }}
                      onClick={e => e.stopPropagation()}
                    >
                      <option value="all">{t.history.recordHoursAll}</option>
                      <option value="custom">{t.history.recordHoursCustom}</option>
                    </select>
                  </div>
                  {hasCustomHours && (
                    <div className="history-hours-row">
                      <select className="history-hours-select" value={s.record_from_hour!} onChange={e => { e.stopPropagation(); onSaveRecordHours(s.id, Number(e.target.value), s.record_to_hour!); }} onClick={e => e.stopPropagation()}>
                        {Array.from({ length: 24 }, (_, i) => <option key={i} value={i}>{String(i).padStart(2, '0')}:00</option>)}
                      </select>
                      <span>–</span>
                      <select className="history-hours-select" value={s.record_to_hour!} onChange={e => { e.stopPropagation(); onSaveRecordHours(s.id, s.record_from_hour!, Number(e.target.value)); }} onClick={e => e.stopPropagation()}>
                        {Array.from({ length: 25 }, (_, i) => <option key={i} value={i}>{i === 24 ? '24:00' : `${String(i).padStart(2, '0')}:00`}</option>)}
                      </select>
                    </div>
                  )}
                  <span className="history-hours-hint priority">{t.history.recordHoursOverride} · {t.history.recordHoursHint} ({tzLabel})</span>
                </div>
              ) : null}
            </div>
          );
        })}
        {!filtered.length && <div className="history-sidebar-empty">{t.common.noResults}</div>}
      </div>
    </div>
  );
}
