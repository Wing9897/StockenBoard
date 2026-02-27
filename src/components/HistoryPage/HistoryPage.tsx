import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { openPath } from '@tauri-apps/plugin-opener';
import { createChart, LineSeries, type IChartApi, type ISeriesApi, type Time } from 'lightweight-charts';
import { getDb } from '../../lib/db';
import { t } from '../../lib/i18n';
import { formatNumber, parsePairFromName } from '../../lib/format';
import { AssetIcon } from '../AssetCard/AssetIcon';
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

/** 本地時區偏移（秒），用於修正 chart 時間軸 */
const TZ_OFFSET_SEC = -new Date().getTimezoneOffset() * 60;

/** 本地時區縮寫，例如 "UTC+8" / "UTC-5" */
const TZ_LABEL = (() => {
  const off = -new Date().getTimezoneOffset();
  const h = Math.floor(Math.abs(off) / 60);
  const m = Math.abs(off) % 60;
  return `UTC${off >= 0 ? '+' : '-'}${h}${m ? ':' + String(m).padStart(2, '0') : ''}`;
})();

function fmtTime(ts: number) { return new Date(ts * 1000).toLocaleString(); }
function label(s: Subscription) { return s.display_name || s.symbol; }
const noop = () => {};

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
  const chartRef = useRef<HTMLDivElement>(null);
  const chartApi = useRef<IChartApi | null>(null);
  const seriesApi = useRef<ISeriesApi<'Line'> | null>(null);

  // ── 載入訂閱 ──
  const loadSubs = useCallback(async () => {
    const db = await getDb();
    setSubs(await db.select<Subscription[]>(
      'SELECT id, sub_type, symbol, display_name, selected_provider_id, asset_type, pool_address, token_from_address, token_to_address, sort_order, record_enabled, record_from_hour, record_to_hour FROM subscriptions ORDER BY sort_order, id'
    ));
  }, []);
  useEffect(() => { loadSubs(); }, [loadSubs]);

  // ── 篩選（支援逗號分隔） ──
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
  const sel = subs.find(s => s.id === selectedId);

  // ── 切換紀錄 ──
  const toggle = useCallback(async (id: number, on: boolean) => {
    await invoke('toggle_record', { subscriptionId: id, enabled: on });
    setSubs(p => p.map(s => s.id === id ? { ...s, record_enabled: on ? 1 : 0 } : s));
  }, []);

  const batchToggle = useCallback(async (on: boolean) => {
    const targets = filtered.filter(s => on ? !s.record_enabled : s.record_enabled);
    if (!targets.length) return;
    for (const s of targets) await invoke('toggle_record', { subscriptionId: s.id, enabled: on });
    const ids = new Set(targets.map(s => s.id));
    setSubs(p => p.map(s => ids.has(s.id) ? { ...s, record_enabled: on ? 1 : 0 } : s));
    onToast.success(t.history.batchDone(targets.length, on));
  }, [filtered, onToast]);

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

  const getPrice = useCallback((r: PriceHistoryRecord) => {
    if (session === 'pre' && r.pre_price != null) return r.pre_price;
    if (session === 'post' && r.post_price != null) return r.post_price;
    return r.price;
  }, [session]);

  const chartData = useMemo(() =>
    records.map(r => ({ time: (r.recorded_at + TZ_OFFSET_SEC) as Time, value: getPrice(r) })),
  [records, getPrice]);

  const chartColor = useCallback(() => {
    const cs = getComputedStyle(document.documentElement);
    if (session === 'pre') return cs.getPropertyValue('--pre-market-color').trim() || 'orange';
    if (session === 'post') return cs.getPropertyValue('--post-market-color').trim() || 'purple';
    return cs.getPropertyValue('--accent').trim() || cs.getPropertyValue('--blue').trim() || 'steelblue';
  }, [session]);

  // ── 圖表 ──
  useEffect(() => {
    if (view !== 'chart' || !chartRef.current) return;
    chartApi.current?.remove();
    chartApi.current = null;
    seriesApi.current = null;
    if (!chartData.length) return;

    const el = chartRef.current;
    const cs = getComputedStyle(document.documentElement);
    const chart = createChart(el, {
      width: el.clientWidth, height: el.clientHeight,
      layout: { background: { color: 'transparent' }, textColor: cs.getPropertyValue('--subtext0').trim() || 'gray' },
      grid: { vertLines: { color: 'rgba(128,128,128,0.1)' }, horzLines: { color: 'rgba(128,128,128,0.1)' } },
      timeScale: { timeVisible: true, secondsVisible: false },
      crosshair: { mode: 0 },
    });
    chartApi.current = chart;
    const s = chart.addSeries(LineSeries, { color: chartColor(), lineWidth: 2 });
    seriesApi.current = s;
    s.setData(chartData);
    chart.timeScale().fitContent();

    const ro = new ResizeObserver(() => chart.applyOptions({ width: el.clientWidth, height: el.clientHeight }));
    ro.observe(el);
    return () => { ro.disconnect(); chart.remove(); chartApi.current = null; seriesApi.current = null; };
  }, [view, chartData, chartColor]);

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
      onToast.success(t.history.cleanupDone(n));
      loadHistory();
    } catch (e) { onToast.error(String(e)); }
  }, [onToast, loadHistory]);

  const openDir = useCallback(async () => {
    try {
      const dir = await invoke<string>('get_data_dir');
      await openPath(dir);
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

  // ── 全頁空狀態 ──
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
      {/* ── Sidebar（收起時完全隱藏） ── */}
      {!collapsed && (
        <div className="h-card history-sidebar">
          <div className="history-sidebar-header">
            <span className="history-sidebar-title">{t.history.title}</span>
            {recCount > 0 && <span className="history-recording-badge">● {recCount}</span>}
            <button className="history-collapse-btn" onClick={() => setCollapsed(true)} title="收起">◀</button>
          </div>

          <div className="hseg equal">
            <button className={filter === 'all' ? 'active' : ''} onClick={() => setFilter('all')}>All</button>
            <button className={filter === 'asset' ? 'active' : ''} onClick={() => setFilter('asset')}>{t.history.spot}</button>
            <button className={filter === 'dex' ? 'active' : ''} onClick={() => setFilter('dex')}>{t.history.dex}</button>
          </div>

          <input className="history-search" type="text" placeholder={t.subs.searchPlaceholder} value={search} onChange={e => setSearch(e.target.value)} />

          <div className="history-batch-row">
            <span className="history-batch-count">{filtRecCount}/{filtered.length}</span>
            <button className="history-batch-btn enable" onClick={() => batchToggle(true)} disabled={allOn}>{t.subs.selectAll}</button>
            <button className="history-batch-btn disable" onClick={() => batchToggle(false)} disabled={filtRecCount === 0}>{t.subs.clearAll}</button>
          </div>

          <div className="history-sub-list">
            {filtered.map(s => {
              const isDex = s.sub_type === 'dex';
              const [from, to] = isDex ? parsePairFromName(s.display_name || s.symbol) : ['', ''];
              const isSelected = selectedId === s.id;
              const hasCustomHours = s.record_from_hour != null && s.record_to_hour != null;
              return (
                <div key={s.id}>
                  <div className={`history-sub-item ${isSelected ? 'selected' : ''}`} onClick={() => setSelectedId(s.id)}>
                    {isDex ? (
                      <div className="history-dex-icons">
                        {from ? <AssetIcon symbol={from} className="asset-icon history-icon" onClick={noop} /> : <div className="asset-icon history-icon"><span className="asset-icon-fallback">?</span></div>}
                        {to ? <AssetIcon symbol={to} className="asset-icon history-icon" onClick={noop} /> : <div className="asset-icon history-icon"><span className="asset-icon-fallback">?</span></div>}
                      </div>
                    ) : (
                      <AssetIcon symbol={s.symbol} className="asset-icon history-icon" onClick={noop} />
                    )}
                    <div className="history-sub-info">
                      <span className="history-sub-symbol">{label(s)}</span>
                      <span className="history-sub-meta">
                        {s.selected_provider_id}
                        {s.record_enabled ? <span className="history-rec-dot">●</span> : null}
                        {hasCustomHours && <span className="history-hours-badge">{s.record_from_hour}–{s.record_to_hour}h</span>}
                      </span>
                    </div>
                    <button
                      className={`history-record-toggle ${s.record_enabled ? 'recording' : ''}`}
                      title={s.record_enabled ? t.history.disableRecord : t.history.enableRecord}
                      onClick={e => { e.stopPropagation(); toggle(s.id, !s.record_enabled); }}
                    >{s.record_enabled ? '●' : ''}</button>
                  </div>
                  {isSelected && s.record_enabled ? (
                    <div className="history-hours-editor">
                      <div className="history-hours-row">
                        <span className="history-hours-label">{t.history.recordHours}</span>
                        <select
                          className="history-hours-select"
                          value={hasCustomHours ? 'custom' : 'all'}
                          onChange={e => {
                          if (e.target.value === 'all') saveRecordHours(s.id, null, null);
                            else saveRecordHours(s.id, 16, 9);
                          }}
                          onClick={e => e.stopPropagation()}
                        >
                          <option value="all">{t.history.recordHoursAll}</option>
                          <option value="custom">{t.history.recordHoursCustom}</option>
                        </select>
                      </div>
                      {hasCustomHours && (
                        <div className="history-hours-row">
                          <select className="history-hours-select" value={s.record_from_hour!} onChange={e => { e.stopPropagation(); saveRecordHours(s.id, Number(e.target.value), s.record_to_hour!); }} onClick={e => e.stopPropagation()}>
                            {Array.from({ length: 24 }, (_, i) => <option key={i} value={i}>{String(i).padStart(2, '0')}:00</option>)}
                          </select>
                          <span>–</span>
                          <select className="history-hours-select" value={s.record_to_hour!} onChange={e => { e.stopPropagation(); saveRecordHours(s.id, s.record_from_hour!, Number(e.target.value)); }} onClick={e => e.stopPropagation()}>
                            {Array.from({ length: 25 }, (_, i) => <option key={i} value={i}>{i === 24 ? '24:00' : `${String(i).padStart(2, '0')}:00`}</option>)}
                          </select>
                        </div>
                      )}
                      <span className="history-hours-hint priority">{t.history.recordHoursOverride} · {t.history.recordHoursHint} ({TZ_LABEL})</span>
                    </div>
                  ) : null}
                </div>
              );
            })}
            {!filtered.length && <div className="history-sidebar-empty">{t.common.noResults}</div>}
          </div>
        </div>
      )}

      {/* ── Main ── */}
      <div className="history-main">
        {/* 工具列（永遠顯示） */}
        <div className="h-card history-toolbar">
          {/* 第一行：資產資訊 + icon 按鈕 */}
          <div className="history-toolbar-row">
            {collapsed && (
              <button className="history-icon-btn" onClick={() => setCollapsed(false)} title="展開側欄">☰</button>
            )}
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
          {/* 第二行：控制項 + icon 按鈕 */}
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
            {showBar && <button className="history-icon-btn" onClick={cleanupSelected} title={t.history.cleanup}>🗑️</button>}
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
          <div className="h-card history-chart-wrapper"><div className="history-chart-container" ref={chartRef} /></div>
        ) : (
          <div className="h-card history-table-container">
            <table className="history-table">
              <thead>
                <tr>
                  <th>{t.history.time} ({TZ_LABEL})</th>
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
        )}
      </div>
    </div>
  );
}
