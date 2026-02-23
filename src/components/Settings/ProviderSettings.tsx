import { useState } from 'react';
import { useProviders } from '../../hooks/useProviders';
import './Settings.css';

const PROVIDER_TYPE_LABELS: Record<string, string> = {
  crypto: '加密貨幣',
  stock: '股票',
  both: '股票+加密',
  prediction: '預測市場',
};

export function ProviderSettings({ onSaved }: { onSaved?: () => void }) {
  const { providers, loading, getProviderInfo, updateProvider, toggleProvider } = useProviders();
  const [editingId, setEditingId] = useState<string | null>(null);
  const [formData, setFormData] = useState<{
    api_key: string; api_secret: string; refresh_interval: number; connection_type: string;
  }>({ api_key: '', api_secret: '', refresh_interval: 30000, connection_type: 'rest' });
  const [filter, setFilter] = useState<string>('all');
  const [useKeyMode, setUseKeyMode] = useState(false);

  const handleEdit = (p: typeof providers[0]) => {
    const hasKey = !!p.api_key;
    setEditingId(p.id);
    setUseKeyMode(hasKey);
    setFormData({
      api_key: p.api_key || '',
      api_secret: p.api_secret || '',
      refresh_interval: p.refresh_interval,
      connection_type: p.connection_type || 'rest',
    });
  };

  const handleModeSwitch = (toKeyMode: boolean) => {
    const info = editingId ? getProviderInfo(editingId) : null;
    setUseKeyMode(toKeyMode);
    if (info) {
      const newInterval = toKeyMode ? info.key_interval : info.free_interval;
      if (!toKeyMode) {
        setFormData(prev => ({ ...prev, api_key: '', api_secret: '', refresh_interval: newInterval }));
      } else {
        setFormData(prev => ({ ...prev, refresh_interval: newInterval }));
      }
    }
  };

  const handleSave = async () => {
    if (!editingId) return;
    const current = providers.find(p => p.id === editingId);
    await updateProvider(editingId, {
      api_key: formData.api_key || null,
      api_secret: formData.api_secret || null,
      refresh_interval: formData.refresh_interval,
      connection_type: formData.connection_type,
      enabled: current?.enabled ?? 1,
    });
    setEditingId(null);
    onSaved?.();
  };

  const filteredProviders = providers.filter(p => filter === 'all' || p.provider_type === filter);

  const showModeToggle = (providerId: string) => {
    const info = getProviderInfo(providerId);
    if (!info) return false;
    return info.requires_api_key || info.optional_api_key;
  };

  const canUseFreeMode = (providerId: string) => {
    const info = getProviderInfo(providerId);
    if (!info) return false;
    return !info.requires_api_key || info.optional_api_key;
  };

  if (loading) return <div className="loading">載入中...</div>;

  return (
    <div className="settings-section">
      <h3>數據源設定 ({providers.length} 個)</h3>
      <p className="settings-hint">所有數據源均可使用，在主頁切換選擇。此處僅設定 API Key 等參數。</p>

      <div className="filter-bar">
        {['all', 'crypto', 'stock', 'both', 'prediction'].map(f => (
          <button key={f} className={`filter-btn ${filter === f ? 'active' : ''}`} onClick={() => setFilter(f)}>
            {f === 'all' ? '全部' : PROVIDER_TYPE_LABELS[f] || f}
          </button>
        ))}
      </div>

      <div className="provider-list">
        {filteredProviders.map(p => {
          const info = getProviderInfo(p.id);
          const isEditing = editingId === p.id;
          const hasKey = !!p.api_key;

          return (
            <div key={p.id} className="provider-item">
              <div className="provider-header">
                <div className="provider-info">
                  <span className="provider-name">{p.name}</span>
                  <span className={`provider-type ${p.provider_type}`}>
                    {PROVIDER_TYPE_LABELS[p.provider_type] || p.provider_type}
                  </span>
                  {showModeToggle(p.id) && (
                    <span className={`badge ${hasKey ? 'api-key-mode' : 'free-mode'}`}>
                      {hasKey ? 'API Key' : '免費'}
                    </span>
                  )}
                  {p.supports_websocket === 1 && <span className="badge ws-support">WebSocket</span>}
                </div>
              </div>

              {info && (
                <div className="provider-meta">
                  <span className="free-tier">{info.free_tier_info}</span>
                  <span className="symbol-format">格式: {info.symbol_format}</span>
                </div>
              )}

              {isEditing ? (
                <div className="provider-form">
                  {showModeToggle(p.id) && (
                    <div className="form-group">
                      <label>使用模式</label>
                      <div className="mode-toggle">
                        {canUseFreeMode(p.id) && (
                          <button type="button" className={`mode-btn ${!useKeyMode ? 'active' : ''}`} onClick={() => handleModeSwitch(false)}>
                            免費版 {info && <span className="mode-interval">{info.free_interval / 1000}秒</span>}
                          </button>
                        )}
                        <button type="button" className={`mode-btn ${useKeyMode ? 'active' : ''}`} onClick={() => handleModeSwitch(true)}>
                          API Key 版 {info && <span className="mode-interval">{info.key_interval / 1000}秒</span>}
                        </button>
                      </div>
                    </div>
                  )}
                  {useKeyMode && (info?.requires_api_key || info?.optional_api_key) && (
                    <div className="form-group">
                      <label>
                        API Key
                        {info?.optional_api_key && !info?.requires_api_key && <span className="optional-badge">提高速率限制</span>}
                      </label>
                      <input type="password" value={formData.api_key} onChange={e => setFormData({ ...formData, api_key: e.target.value })} placeholder="輸入 API Key" />
                    </div>
                  )}
                  {useKeyMode && info?.requires_api_secret && (
                    <div className="form-group">
                      <label>API Secret</label>
                      <input type="password" value={formData.api_secret} onChange={e => setFormData({ ...formData, api_secret: e.target.value })} placeholder="輸入 API Secret" />
                    </div>
                  )}
                  <div className="form-group">
                    <label>
                      刷新間隔 (毫秒)
                      {info && <span className="optional-badge">建議: {(useKeyMode ? info.key_interval : info.free_interval) / 1000}秒</span>}
                    </label>
                    <input type="number" value={formData.refresh_interval} onChange={e => setFormData({ ...formData, refresh_interval: parseInt(e.target.value) })} min={5000} step={1000} />
                  </div>
                  {p.supports_websocket === 1 && (
                    <div className="form-group">
                      <label>連接方式</label>
                      <select value={formData.connection_type} onChange={e => setFormData({ ...formData, connection_type: e.target.value })}>
                        <option value="rest">REST API</option>
                        <option value="websocket">WebSocket</option>
                      </select>
                    </div>
                  )}
                  <div className="form-actions">
                    <button className="btn-cancel" onClick={() => setEditingId(null)}>取消</button>
                    <button className="btn-save" onClick={handleSave}>儲存</button>
                  </div>
                </div>
              ) : (
                <div className="provider-details">
                  <span>刷新: {p.refresh_interval / 1000}秒</span>
                  <span>連接: {p.connection_type === 'websocket' ? 'WebSocket' : 'REST'}</span>
                  {p.api_key && <span className="api-status">🔑 已設定</span>}
                  <button className="btn-edit" onClick={() => handleEdit(p)}>編輯</button>
                  <button
                    className={`btn-toggle ${p.enabled === 1 ? 'enabled' : 'disabled'}`}
                    onClick={() => toggleProvider(p.id, p.enabled !== 1)}
                    title={p.enabled === 1 ? '停用此數據源' : '啟用此數據源'}
                  >
                    {p.enabled === 1 ? '啟用' : '停用'}
                  </button>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
