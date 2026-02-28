/**
 * 歷史頁面 — 精簡版，子元件已抽出到 HistorySidebar / HistoryChart / HistoryTable
 */
import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getDb } from '../../lib/db';
import { t } from '../../lib/i18n';
import { HistorySidebar } from './HistorySidebar';
import { HistoryChart } from './HistoryChart';
import { HistoryTable } from './HistoryTable';
import type { Subscription, PriceHistoryRecord } from '../../types';
import './HistoryPage.css';

type ViewType = 'chart' | 'table';
type RangePreset = '1d' | '1w' | '1m' | '1y' | 'custom';
type SubFilter = 'all' | 'asset' | 'dex';
type SessionFilter = 'regular' | 'pre' | 'post';

interface Props {
  onToast: { success: (title: string, msg?: string) => void; error: (title: string, msg?: string) => void; info: (title: string, msg?: string) => void };
}

const DAY = 86400;
const RANGE_MAP: Record<Exclude<RangePreset, 'custom'>, number> = { '1d': DAY, '1w': 7 * DAY, '1m': 30 * DAY, '1y': 365 * DAY };
const RANGE_LABELS: { key: RangePreset; label: () => string }[] = [
  { key: '1d', label: () => t.history.day1 },
  { key: '1w', label: () => t.history.week1 },
  { key: '1m', label: () => t.history.month1 },
  { key: '1y', label: () => t.history.year1 },
  { key: 'custom', label: () => t.history.custom },
];

const TZ_LABEL = (() => {
  const off = -new Date().getTimezoneOffset();
  const h = Math.floor(Math.abs(off) / 60);
  const m = Math.abs(off) % 60;
  return `UTC${off >= 0 ? '+' : '-'}${h}${m ? ':' + String(m).padStart(2, '0') : ''}`;
})();

function label(s: Subscription) { return s.display_name || s.symbol; }

export function HistoryPage({ onToast }: Props) {
  const [subs, setSubs] = useState<Subscription[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [view, setView] = useState<ViewType>('chart');
  const [range, setRange] = useState<RangePreset>('1d');
  const [customFrom, setCustomFrom] = useState('');
  const [customTo, setCustomTo] = useState('');
  const [records, setRecords] = useState<PriceHistoryRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [filter, setFilter] = useState<SubFilter>('all');
  const [search, setSearch] = useState('');
  const [session, setSession] = useState<SessionFilter>('regular');
  const [collapsed, setCollapsed] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  // ── 載入訂閱 ──
  const loadSubs = useCallback(async () => {
    const db = await getDb();
    setSubs(await db.select<Subscription[]>(
      'SELECT id, sub_type, symbol, display_name, selected_provider_id, asset_type, pool_address, token_from_address, token_to_address, sort_order, record_enabled, record_from_hour, record_to_hour FROM subscriptions ORDER BY sort_order, id'
    ));
  }, []);
  useEffect(() => { loadSubs(); }, [loadSubs]);

  const sel = subs.find(s => s.id === selectedId);

  // ── 切換紀錄 ──
  const toggle = useCallback(async (id: number, on: boolean) => {
    await invoke('toggle_record', { subscriptionId: id, enabled: on });
    setSubs(p => p.map(s => s.id === id ? { ...s, record_enabled: on ? 1 : 0 } : s));
  }, []);

  const batchToggle = useCallback(async (on: boolean) => {
    // 篩選當前可見的訂閱
    let list = subs;
    if (filter === 'dex') list = list.filter(s => s.sub_type === 'dex');
    else if (filter === 'asset') list = list.filter(s => s.sub_type === 'asset');
    const kw = search.split(/[,，;；]/).map(k => k.trim().toLowerCase()).filter(Boolean);
    if (kw.length) list = list.filter(s => kw.some(q => `${s.display_name || ''} ${s.symbol} ${s.selected_provider_id}`.toLowerCase().includes(q)));
    const targets = list.filter(s => on ? !s.record_enabled : s.record_enabled);
    if (!targets.length) return;
    for (const s of targets) await invoke('toggle_record', { subscriptionId: s.id, enabled: on });
    const ids = new Set(targets.map(s => s.id));
    setSubs(p => p.map(s => ids.has(s.id) ? { ...s, record_enabled: on ? 1 : 0 } : s));
    onToast.success(t.history.batchDone(targets.length, on));
  }, [subs, filter, search, onToast]);

  // ── 紀錄時段 ──
  const saveRecordHours = useCallback(async (id: number, from: number | null, to: number | null) => {
    await invoke('set_record_hours', { subscriptionId: id, fromHour: from, toHour: to });
    setSubs(p => p.map(s => s.id === id ? { ...s, record_from_hour: from, record_to_hour: to } : s));
    onToast.success(t.history.recordHoursSaved);
  }, [onToast]);

  // ── 載入歷史 ──
  const loadHistory = useCallback(async () => {
    if (!selectedId) return;
    setLoading(true);
    try {
      const now = Math.floor(Date.now() / 1000);
      let fromTs: number, toTs: number;
      if (range === 'custom' && customFrom && customTo) {
        fromTs = Math.floor(new Date(customFrom).getTime() / 1000);
        toTs = Math.floor(new Date(customTo + 'T23:59:59').getTime() / 1000);
      } else {
        fromTs = now - (RANGE_MAP[range as keyof typeof RANGE_MAP] || DAY);
        toTs = now;
      }
      setRecords(await invoke<PriceHistoryRecord[]>('get_price_history', { subscriptionId: selectedId, fromTs, toTs, limit: 10000 }));
    } catch (e) {
      console.error('loadHistory:', e);
      setRecords([]);
    } finally { setLoading(false); }
  }, [selectedId, range, customFrom, customTo]);
  useEffect(() => { loadHistory(); }, [loadHistory]);

  // ── Session 數據 ──
  const hasPre = useMemo(() => records.some(r => r.pre_price != null), [records]);
  const hasPost = useMemo(() => records.some(r => r.post_price != null), [records]);
  const hasSession = hasPre || hasPost;

  // ── 清理 ──
  const cleanupSelected = useCallback(async () => {
    if (!selectedId) return;
    if (!confirm(t.history.purgeAllConfirm)) return;
    try {
      const db = await getDb();
      const r = await db.execute('DELETE FROM price_history WHERE subscription_id = ?', [selectedId]);
      onToast.success(t.history.purgeAllDone(r.rowsAffected));
      setRecords([]);
    } catch (e) { onToast.error(String(e)); }
  }, [selectedId, onToast]);

  const purgeAll = useCallback(async () => {
    if (!confirm(t.history.purgeAllConfirm)) return;
    try {
      const n = await invoke<number>('purge_all_history');
      onToast.success(t.history.purgeAllDone(n));
      setRecords([]);
    } catch (e) { onToast.error(String(e)); }
  }, [onToast]);

  const cleanup90 = useCallback(async () => {
    try {
      const n = await invoke<number>('cleanup_history', { retentionDays: 90 });
      if (n === 0) {
        onToast.info(t.history.noOldData);
      } else {
        onToast.success(t.history.cleanupDone(n));
      }
      loadHistory();
    } catch (e) { onToast.error(String(e)); }
  }, [onToast, loadHistory]);

  const openDir = useCallback(async () => {
    try {
      await invoke<string>('get_data_dir');
      onToast.success(t.history.openDataDir);
    } catch (e) { onToast.error(String(e)); }
  }, [onToast]);

  // ── 點擊外部關閉選單 ──
  useEffect(() => {
    if (!menuOpen) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) setMenuOpen(false);
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [menuOpen]);

  const hasData = records.length > 0;
  const showBar = selectedId && hasData && !loading;

  if (!subs.length) return (
    <div className="history-page">
      <div className="h-card history-full-empty">
        <div className="history-empty-icon">📊</div>
        <p>{t.history.noSubs}</p>
      </div>
    </div>
  );

  return (
    <div className="history-page">
      {!collapsed && (
        <HistorySidebar
          subs={subs}
          selectedId={selectedId}
          filter={filter}
          search={search}
          onSelectId={setSelectedId}
          onSetFilter={setFilter}
          onSetSearch={setSearch}
          onToggle={toggle}
          onBatchToggle={batchToggle}
          onCollapse={() => setCollapsed(true)}
          onSaveRecordHours={saveRecordHours}
          tzLabel={TZ_LABEL}
        />
      )}

      <div className="history-main">
        <div className="h-card history-toolbar">
          <div className="history-toolbar-row">
            {collapsed && <button className="history-icon-btn" onClick={() => setCollapsed(false)} title="展開側欄">☰</button>}
            {sel && (
              <>
                <span className="history-selected-name">{label(sel)}</span>
                <span className="history-selected-provider">{sel.selected_provider_id}</span>
                <span className={`history-status-badge ${sel.record_enabled ? 'recording' : ''}`}>
                  {sel.record_enabled ? t.history.recording : t.history.notRecording}
                </span>
              </>
            )}
            {!sel && !collapsed && <span className="history-toolbar-hint">{t.history.selectSub}</span>}
          </div>
          <div className="history-toolbar-row">
            {showBar && (
              <>
                <div className="hseg lg">
                  <button className={view === 'chart' ? 'active' : ''} onClick={() => setView('chart')}>{t.history.chartView}</button>
                  <button className={view === 'table' ? 'active' : ''} onClick={() => setView('table')}>{t.history.tableView}</button>
                </div>
                <div className="history-toolbar-divider" />
                <div className="hseg">
                  {RANGE_LABELS.map(r => (
                    <button key={r.key} className={range === r.key ? 'active' : ''} onClick={() => setRange(r.key)}>{r.label()}</button>
                  ))}
                </div>
                {range === 'custom' && (
                  <div className="history-custom-range">
                    <span>{t.history.from}</span>
                    <input type="date" className="history-date-input" value={customFrom} onChange={e => setCustomFrom(e.target.value)} />
                    <span>{t.history.to}</span>
                    <input type="date" className="history-date-input" value={customTo} onChange={e => setCustomTo(e.target.value)} />
                    <button className="history-apply-btn" onClick={loadHistory}>{t.history.apply}</button>
                  </div>
                )}
                {hasSession && (
                  <>
                    <div className="history-toolbar-divider" />
                    <div className="hseg">
                      {hasPre && <button className={session === 'pre' ? 'active' : ''} onClick={() => setSession('pre')}>{t.history.prePrice}</button>}
                      <button className={session === 'regular' ? 'active' : ''} onClick={() => setSession('regular')}>{t.asset.sessionRegular}</button>
                      {hasPost && <button className={session === 'post' ? 'active' : ''} onClick={() => setSession('post')}>{t.history.postPrice}</button>}
                    </div>
                  </>
                )}
              </>
            )}
            <div className="history-spacer" />
            {showBar && <span className="history-stats">{t.history.records(records.length)} · {TZ_LABEL}</span>}
            {showBar && <button className="history-icon-btn" onClick={cleanupSelected} title={t.history.cleanupCurrent}>🗑️</button>}
            <div className="history-menu-wrap" ref={menuRef}>
              <button className="history-icon-btn" onClick={() => setMenuOpen(v => !v)} title={t.nav.settings}>⚙️</button>
              {menuOpen && (
                <div className="history-menu">
                  <button onClick={() => { cleanup90(); setMenuOpen(false); }}>{t.history.cleanup}</button>
                  <button className="danger" onClick={() => { purgeAll(); setMenuOpen(false); }}>{t.history.purgeAll}</button>
                  <button onClick={() => { openDir(); setMenuOpen(false); }}>{t.history.openDataDir}</button>
                </div>
              )}
            </div>
          </div>
        </div>

        {!selectedId ? (
          <div className="h-card history-empty-state">
            <div className="history-empty-icon">👈</div>
            <p>{t.history.selectSub}</p>
          </div>
        ) : loading ? (
          <div className="h-card history-empty-state"><p>{t.common.loading}</p></div>
        ) : !hasData ? (
          <div className="h-card history-empty-state">
            <div className="history-empty-icon">📭</div>
            <p>{t.history.noData}</p>
            {!sel?.record_enabled && (
              <button className="history-enable-btn" onClick={() => sel && toggle(sel.id, true)}>{t.history.enableRecord}</button>
            )}
          </div>
        ) : view === 'chart' ? (
          <HistoryChart records={records} session={session} />
        ) : (
          <HistoryTable records={records} session={session} tzLabel={TZ_LABEL} />
        )}
      </div>
    </div>
  );
}
