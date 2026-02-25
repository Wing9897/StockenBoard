import { useState, useMemo } from 'react';
import { useProviders } from '../../hooks/useProviders';
import { t } from '../../lib/i18n';
import { useLocale } from '../../hooks/useLocale';
import './Settings.css';

type SettingsViewMode = 'grid' | 'list';

const TYPE_COLORS: Record<string, string> = {
  crypto: 'var(--peach)',
  stock: 'var(--blue)',
  both: 'var(--mauve)',
  prediction: 'var(--teal)',
  dex: 'var(--yellow)',
};

const TYPE_BG: Record<string, string> = {
  crypto: 'var(--peach-bg)',
  stock: 'var(--blue-bg)',
  both: 'var(--mauve-bg)',
  prediction: 'var(--teal-bg)',
  dex: 'var(--yellow-bg)',
};

export function ProviderSettings({ onSaved }: { onSaved?: () => void }) {
  useLocale();
  const TYPE_LABELS: Record<string, string> = {
    crypto: t.providers.crypto,
    stock: t.providers.stock,
    both: t.providers.both,
    prediction: t.providers.prediction,
    dex: t.providers.dex,
  };
  const TYPE_FILTERS = [
    { key: 'all', label: t.providers.all },
    { key: 'crypto', label: t.providers.crypto },
    { key: 'stock', label: t.providers.stock },
    { key: 'both', label: t.providers.both },
    { key: 'dex', label: t.providers.dex },
    { key: 'prediction', label: t.providers.prediction },
  ];
  const { providers, loading, getProviderInfo, updateProvider } = useProviders();
  const [search, setSearch] = useState('');
  const [filter, setFilter] = useState('all');
  const [viewMode, setViewMode] = useState<SettingsViewMode>(() => {
    const saved = localStorage.getItem('sb_settings_view');
    return saved === 'list' ? 'list' : 'grid';
  });
  const [editingId, setEditingId] = useState<string | null>(null);
  const [formData, setFormData] = useState({
    api_key: '', api_secret: '', api_url: '',
    refresh_interval: 30000, connection_type: 'rest',
  });
  const [useKeyMode, setUseKeyMode] = useState(false);

  const handleSetView = (m: SettingsViewMode) => {
    setViewMode(m);
    localStorage.setItem('sb_settings_view', m);
  };

  const filtered = useMemo(() => {
    let list = providers;
    if (filter !== 'all') list = list.filter(p => p.provider_type === filter);
    if (search.trim()) {
      const q = search.toLowerCase();
      list = list.filter(p =>
        p.name.toLowerCase().includes(q) || p.id.toLowerCase().includes(q) ||
        (TYPE_LABELS[p.provider_type] || '').includes(q)
      );
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
    });
    setEditingId(null);
    onSaved?.();
  };

  const showModeToggle = (pid: string) => {
    const info = getProviderInfo(pid);
    return info ? (info.requires_api_key || info.optional_api_key) : false;
  };
  const canUseFree = (pid: string) => {
    const info = getProviderInfo(pid);
    return info ? (!info.requires_api_key || info.optional_api_key) : false;
  };

  if (loading) return <div className="loading">{t.common.loading}</div>;
  const editingProvider = editingId ? providers.find(p => p.id === editingId) : null;
  const editInfo = editingId ? getProviderInfo(editingId) : null;
  const getDesc = (id: string) => {
    const desc = (t.providerDesc as Record<string, string>)?.[id];
    return desc || getProviderInfo(id)?.free_tier_info || '';
  };

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
            <input className="ps-search" type="text" placeholder={t.providers.searchPlaceholder} value={search} onChange={e => setSearch(e.target.value)} />
            {search && <button className="ps-search-clear" onClick={() => setSearch('')}>&#x2715;</button>}
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
        <div className="modal-backdrop ps-modal-backdrop" onClick={() => setEditingId(null)}>
          <div className="modal-container ps-modal" onClick={e => e.stopPropagation()}>
            <div className="ps-modal-head">
              <div>
                <h4 className="ps-modal-title">{editingProvider.name}</h4>
                <span className="ps-modal-type" style={{ color: TYPE_COLORS[editingProvider.provider_type] }}>{TYPE_LABELS[editingProvider.provider_type]}</span>
              </div>
              <button className="ps-modal-close" onClick={() => setEditingId(null)}>&#x2715;</button>
            </div>
            {editInfo && (
              <div className="ps-modal-meta">
                <div className="ps-meta-item"><span className="ps-meta-label">{t.providers.plan}</span><span className="ps-meta-value">{getDesc(editingId)}</span></div>
                <div className="ps-meta-item"><span className="ps-meta-label">{t.providers.connection}</span><span className="ps-meta-value">{editingProvider?.connection_type === 'websocket' ? t.providers.websocket : t.providers.restApi}</span></div>
                <div className="ps-meta-item"><span className="ps-meta-label">{t.providers.format}</span><span className="ps-meta-value mono">{editInfo.symbol_format}</span></div>
              </div>
            )}
            <div className="ps-modal-body">
              {showModeToggle(editingId) && (
                <div className="form-group">
                  <label>{t.providers.useMode}</label>
                  <div className="mode-toggle">
                    {canUseFree(editingId) && (
                      <button type="button" className={`mode-btn ${!useKeyMode ? 'active' : ''}`} onClick={() => handleModeSwitch(false)}>
                        {t.providers.freeMode} {editInfo && <span className="mode-interval">{editInfo.free_interval / 1000}{t.providers.seconds}</span>}
                      </button>
                    )}
                    <button type="button" className={`mode-btn ${useKeyMode ? 'active' : ''}`} onClick={() => handleModeSwitch(true)}>
                      {t.providers.apiKeyMode} {editInfo && <span className="mode-interval">{editInfo.key_interval / 1000}{t.providers.seconds}</span>}
                    </button>
                  </div>
                </div>
              )}
              {useKeyMode && (editInfo?.requires_api_key || editInfo?.optional_api_key) && (
                <div className="form-group">
                  <label>{t.apiKey.label} {editInfo?.optional_api_key && !editInfo?.requires_api_key && <span className="optional-badge">{t.providers.boostRate}</span>}</label>
                  <input type="password" value={formData.api_key} onChange={e => setFormData({ ...formData, api_key: e.target.value })} placeholder={t.apiKey.placeholder} />
                </div>
              )}
              {useKeyMode && editInfo?.requires_api_secret && (
                <div className="form-group">
                  <label>{t.apiKey.secretLabel}</label>
                  <input type="password" value={formData.api_secret} onChange={e => setFormData({ ...formData, api_secret: e.target.value })} placeholder={t.apiKey.secretPlaceholder} />
                </div>
              )}
              {editInfo?.provider_type === 'dex' && (
                <div className="form-group">
                  <label>{t.providers.apiUrl} <span className="optional-badge">{t.providers.apiUrlOptional}</span></label>
                  <input value={formData.api_url} onChange={e => setFormData({ ...formData, api_url: e.target.value })} placeholder={t.providers.apiUrlPlaceholder} />
                </div>
              )}
              <div className="form-group">
                <label>{t.providers.refreshInterval} {editInfo && <span className="optional-badge">{t.providers.refreshHint((useKeyMode ? editInfo.key_interval : editInfo.free_interval) / 1000)}</span>}</label>
                <input type="number" value={formData.refresh_interval} onChange={e => setFormData({ ...formData, refresh_interval: parseInt(e.target.value) || 5000 })} min={5000} step={1000} />
              </div>
              {editingProvider.supports_websocket === 1 && (
                <div className="form-group">
                  <label>{t.providers.connectionMethod}</label>
                  <select value={formData.connection_type} onChange={e => setFormData({ ...formData, connection_type: e.target.value })}>
                    <option value="rest">{t.providers.restApi}</option>
                    <option value="websocket">{t.providers.websocket}</option>
                  </select>
                </div>
              )}
            </div>
            <div className="ps-modal-foot">
              <button className="btn-cancel" onClick={() => setEditingId(null)}>{t.common.cancel}</button>
              <button className="btn-save" onClick={handleSave}>{t.common.save}</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
