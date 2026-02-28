/**
 * Provider 設定頁面 — 精簡版 orchestrator
 * Grid/List 渲染 inline，Modal 抽到 ProviderModal.tsx
 */
import { useState, useMemo } from 'react';
import { useProviders } from '../../hooks/useProviders';
import { useEscapeKey } from '../../hooks/useEscapeKey';
import { useLocale } from '../../hooks/useLocale';
import { TYPE_COLORS, TYPE_BG, getTypeLabels, getTypeFilters } from './providerConstants';
import { ProviderModal } from './ProviderModal';
import { t } from '../../lib/i18n';
import './Settings.css';

type SettingsViewMode = 'grid' | 'list';

export function ProviderSettings({ onSaved }: { onSaved?: () => void }) {
  useLocale();
  const TYPE_LABELS = getTypeLabels();
  const TYPE_FILTERS = getTypeFilters();
  const { providers, loading, getProviderInfo, updateProvider } = useProviders();
  const [search, setSearch] = useState('');
  const [filter, setFilter] = useState('all');
  const [viewMode, setViewMode] = useState<SettingsViewMode>(() =>
    localStorage.getItem('sb_settings_view') === 'list' ? 'list' : 'grid'
  );
  const [editingId, setEditingId] = useState<string | null>(null);
  const [formData, setFormData] = useState({
    api_key: '', api_secret: '', api_url: '',
    refresh_interval: 30000, connection_type: 'rest',
    record_from_hour: null as number | null, record_to_hour: null as number | null,
  });
  const [useKeyMode, setUseKeyMode] = useState(false);

  const handleSetView = (m: SettingsViewMode) => { setViewMode(m); localStorage.setItem('sb_settings_view', m); };

  const filtered = useMemo(() => {
    let list = providers;
    if (filter !== 'all') list = list.filter(p => p.provider_type === filter);
    if (search.trim()) {
      const q = search.toLowerCase();
      list = list.filter(p => p.name.toLowerCase().includes(q) || p.id.toLowerCase().includes(q) || (TYPE_LABELS[p.provider_type] || '').includes(q));
    }
    return list;
  }, [providers, filter, search]);

  const counts = useMemo(() => {
    const c: Record<string, number> = { all: providers.length };
    for (const p of providers) c[p.provider_type] = (c[p.provider_type] || 0) + 1;
    return c;
  }, [providers]);

  const openEdit = (p: typeof providers[0]) => {
    setEditingId(p.id);
    setUseKeyMode(!!p.api_key);
    setFormData({
      api_key: p.api_key || '', api_secret: p.api_secret || '', api_url: p.api_url || '',
      refresh_interval: p.refresh_interval, connection_type: p.connection_type || 'rest',
      record_from_hour: p.record_from_hour ?? null, record_to_hour: p.record_to_hour ?? null,
    });
  };

  const handleModeSwitch = (toKey: boolean) => {
    const info = editingId ? getProviderInfo(editingId) : null;
    setUseKeyMode(toKey);
    if (info) {
      const iv = toKey ? info.key_interval : info.free_interval;
      if (!toKey) setFormData(prev => ({ ...prev, api_key: '', api_secret: '', refresh_interval: iv }));
      else setFormData(prev => ({ ...prev, refresh_interval: iv }));
    }
  };

  const handleSave = async () => {
    if (!editingId) return;
    await updateProvider(editingId, {
      api_key: formData.api_key || null, api_secret: formData.api_secret || null,
      api_url: formData.api_url || null, refresh_interval: formData.refresh_interval,
      connection_type: formData.connection_type,
      record_from_hour: formData.record_from_hour, record_to_hour: formData.record_to_hour,
    });
    setEditingId(null);
    onSaved?.();
  };

  useEscapeKey(() => { if (editingId) setEditingId(null); });

  if (loading) return <div className="loading">{t.common.loading}</div>;

  const editingProvider = editingId ? providers.find(p => p.id === editingId) : null;
  const editInfo = editingId ? getProviderInfo(editingId) : undefined;
  const getDesc = (id: string) => (t.providerDesc as Record<string, string>)?.[id] || getProviderInfo(id)?.free_tier_info || '';

  return (
    <div className="ps-section">
      <div className="ps-toolbar">
        <div className="ps-toolbar-left">
          <h3 className="ps-title">{t.providers.title}</h3>
          <span className="ps-count">{filtered.length}/{providers.length}</span>
        </div>
        <div className="ps-toolbar-right">
          <div className="ps-search-wrap">
            <svg className="ps-search-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
            <input className="ps-search" type="text" placeholder={t.providers.searchPlaceholder} value={search} onChange={e => setSearch(e.target.value)} aria-label={t.providers.searchPlaceholder} />
            {search && <button className="ps-search-clear" onClick={() => setSearch('')} aria-label={t.common.clearSearch}>&#x2715;</button>}
          </div>
          <div className="ps-view-toggle">
            <button className={`ps-vbtn ${viewMode === 'grid' ? 'active' : ''}`} onClick={() => handleSetView('grid')} title={t.viewMode.grid}>
              <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><rect x="1" y="1" width="6" height="6" rx="1"/><rect x="9" y="1" width="6" height="6" rx="1"/><rect x="1" y="9" width="6" height="6" rx="1"/><rect x="9" y="9" width="6" height="6" rx="1"/></svg>
            </button>
            <button className={`ps-vbtn ${viewMode === 'list' ? 'active' : ''}`} onClick={() => handleSetView('list')} title={t.viewMode.list}>
              <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><rect x="1" y="2" width="14" height="2.5" rx="1"/><rect x="1" y="6.75" width="14" height="2.5" rx="1"/><rect x="1" y="11.5" width="14" height="2.5" rx="1"/></svg>
            </button>
          </div>
        </div>
      </div>
      <div className="ps-chips">
        {TYPE_FILTERS.map(f => (
          <button key={f.key} className={`ps-chip ${filter === f.key ? 'active' : ''}`} onClick={() => setFilter(f.key)}
            style={filter === f.key && f.key !== 'all' ? { background: TYPE_BG[f.key], color: TYPE_COLORS[f.key], borderColor: TYPE_COLORS[f.key] } : undefined}
          >
            {f.label}
            {counts[f.key] != null && <span className="ps-chip-count">{counts[f.key]}</span>}
          </button>
        ))}
      </div>

      {filtered.length === 0 ? (
        <div className="ps-empty"><span className="ps-empty-icon">&#x1F50D;</span><span>{t.providers.noMatch}</span></div>
      ) : viewMode === 'grid' ? (
        <div className="ps-grid">
          {filtered.map(p => {
            const info = getProviderInfo(p.id);
            const hasKey = !!p.api_key;
            const color = TYPE_COLORS[p.provider_type] || 'var(--text)';
            return (
              <div key={p.id} className="ps-card" onClick={() => openEdit(p)} style={{ '--accent': color } as React.CSSProperties}>
                <div className="ps-card-accent" />
                <div className="ps-card-body">
                  <div className="ps-card-head">
                    <span className="ps-card-name">{p.name}</span>
                    <span className="ps-card-type" style={{ color }}>{TYPE_LABELS[p.provider_type]}</span>
                  </div>
                  <div className="ps-card-tags">
                    {hasKey ? <span className="ps-tag key">{t.providers.apiKey}</span> : <span className="ps-tag free">{t.providers.free}</span>}
                    <span className={`ps-tag ${p.connection_type === 'websocket' ? 'ws' : 'rest'}`}>{p.connection_type === 'websocket' ? t.providers.websocket : t.providers.restApi}</span>
                  </div>
                  {info && <p className="ps-card-desc">{getDesc(p.id)}</p>}
                  <div className="ps-card-foot">
                    <span>{p.refresh_interval / 1000}{t.providers.seconds}</span>
                    {info && <span className="ps-card-fmt">{info.symbol_format}</span>}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        <div className="ps-table">
          <div className="ps-table-head">
            <span className="ps-th name">{t.common.name}</span>
            <span className="ps-th type">{t.common.type}</span>
            <span className="ps-th status">{t.providers.connection}</span>
            <span className="ps-th tier">{t.providers.plan}</span>
            <span className="ps-th interval">{t.providers.interval}</span>
            <span className="ps-th fmt">{t.providers.format}</span>
          </div>
          {filtered.map(p => {
            const info = getProviderInfo(p.id);
            const hasKey = !!p.api_key;
            const color = TYPE_COLORS[p.provider_type] || 'var(--text)';
            return (
              <div key={p.id} className="ps-table-row" onClick={() => openEdit(p)}>
                <span className="ps-td name">{p.name}</span>
                <span className="ps-td type" style={{ color }}>{TYPE_LABELS[p.provider_type]}</span>
                <span className="ps-td status">
                  {hasKey ? <span className="ps-tag key">&#x1F511;</span> : <span className="ps-tag free">{t.providers.free}</span>}
                  <span className={`ps-tag ${p.connection_type === 'websocket' ? 'ws' : 'rest'}`}>{p.connection_type === 'websocket' ? t.providers.websocket : t.providers.restApi}</span>
                </span>
                <span className="ps-td tier">{getDesc(p.id)}</span>
                <span className="ps-td interval">{p.refresh_interval / 1000}{t.providers.seconds}</span>
                <span className="ps-td fmt">{info?.symbol_format}</span>
              </div>
            );
          })}
        </div>
      )}

      {editingId && editingProvider && (
        <ProviderModal
          provider={editingProvider}
          info={editInfo}
          formData={formData}
          useKeyMode={useKeyMode}
          getDesc={getDesc}
          showModeToggle={(() => { const i = getProviderInfo(editingId); return i ? (i.requires_api_key || i.optional_api_key) : false; })()}
          canUseFree={(() => { const i = getProviderInfo(editingId); return i ? (!i.requires_api_key || i.optional_api_key) : false; })()}
          onFormChange={setFormData}
          onModeSwitch={handleModeSwitch}
          onSave={handleSave}
          onClose={() => setEditingId(null)}
        />
      )}
    </div>
  );
}
