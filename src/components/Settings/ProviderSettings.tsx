import { useState } from 'react';
import { Provider } from '../../types';
import { useProviders } from '../../hooks/useProviders';
import './Settings.css';

const PROVIDER_TYPE_LABELS: Record<string, string> = {
  crypto: '加密貨幣',
  stock: '股票',
  both: '股票+加密',
  prediction: '預測市場',
};

export function ProviderSettings({ onSaved }: { onSaved?: () => void }) {
  const { providers, updateProvider, loading, getProviderInfo } = useProviders();
  const [editingId, setEditingId] = useState<string | null>(null);
  const [formData, setFormData] = useState<Partial<Provider>>({});
  const [filter, setFilter] = useState<string>('all');
  const [useKeyMode, setUseKeyMode] = useState<boolean>(false);

  const handleEdit = (provider: Provider) => {
    const info = getProviderInfo(provider.id);
    const hasKey = !!provider.api_key;
    setEditingId(provider.id);
    setUseKeyMode(hasKey);
    setFormData({
      api_key: provider.api_key || '',
      api_secret: provider.api_secret || '',
      refresh_interval: provider.refresh_interval,
      connection_type: provider.connection_type || 'rest',
    });
    // If no custom interval was set, use the appropriate default
    if (info && provider.refresh_interval === (hasKey ? info.key_interval : info.free_interval)) {
      setFormData(prev => ({ ...prev, refresh_interval: provider.refresh_interval }));
    }
  };

  const handleModeSwitch = (toKeyMode: boolean) => {
    const info = editingId ? getProviderInfo(editingId) : null;
    setUseKeyMode(toKeyMode);
    if (info) {
      const newInterval = toKeyMode ? info.key_interval : info.free_interval;
      if (!toKeyMode) {
        // Switching to free: clear API key and set free interval
        setFormData(prev => ({ ...prev, api_key: '', api_secret: '', refresh_interval: newInterval }));
      } else {
        // Switching to key mode: set key interval, keep existing key
        setFormData(prev => ({ ...prev, refresh_interval: newInterval }));
      }
    }
  };

  const handleSave = async () => {
    if (!editingId) return;
    await updateProvider({ id: editingId, ...formData });
    setEditingId(null);
    onSaved?.();
  };

  const filteredProviders = providers.filter((p) => {
    if (filter === 'all') return true;
    return p.provider_type === filter;
  });

  // Determine if a provider should show the free/key mode toggle
  // Show toggle when: requires_api_key (always needs key) OR optional_api_key (can use free or key)
  // Don't show for: providers that are always free and have no key option (binance, coinbase, yahoo, polymarket)
  const showModeToggle = (providerId: string) => {
    const info = getProviderInfo(providerId);
    if (!info) return false;
    return info.requires_api_key || info.optional_api_key;
  };

  // For providers that require API key, "free mode" means they can't actually fetch
  // but we still let them see the interval difference
  const canUseFreeMode = (providerId: string) => {
    const info = getProviderInfo(providerId);
    if (!info) return false;
    // Only truly free-capable: optional_api_key providers (coingecko, cryptocompare)
    // and providers that don't require a key at all
    return !info.requires_api_key || info.optional_api_key;
  };

  if (loading) return <div className="loading">載入中...</div>;

  return (
    <div className="settings-section">
      <h3>數據源設定 ({providers.length} 個)</h3>
      <p className="settings-hint">所有數據源均可使用，在主頁切換選擇。此處僅設定 API Key 等參數。</p>

      <div className="filter-bar">
        <button className={`filter-btn ${filter === 'all' ? 'active' : ''}`} onClick={() => setFilter('all')}>
          全部
        </button>
        <button className={`filter-btn ${filter === 'crypto' ? 'active' : ''}`} onClick={() => setFilter('crypto')}>
          加密貨幣
        </button>
        <button className={`filter-btn ${filter === 'stock' ? 'active' : ''}`} onClick={() => setFilter('stock')}>
          股票
        </button>
        <button className={`filter-btn ${filter === 'both' ? 'active' : ''}`} onClick={() => setFilter('both')}>
          股票+加密
        </button>
        <button className={`filter-btn ${filter === 'prediction' ? 'active' : ''}`} onClick={() => setFilter('prediction')}>
          預測市場
        </button>
      </div>

      <div className="provider-list">
        {filteredProviders.map((provider) => {
          const info = getProviderInfo(provider.id);
          const isEditing = editingId === provider.id;
          const hasKey = !!provider.api_key;
          const currentMode = hasKey ? 'API Key' : '免費';

          return (
            <div key={provider.id} className="provider-item">
              <div className="provider-header">
                <div className="provider-info">
                  <span className="provider-name">{provider.name}</span>
                  <span className={`provider-type ${provider.provider_type}`}>
                    {PROVIDER_TYPE_LABELS[provider.provider_type] || provider.provider_type}
                  </span>
                  {showModeToggle(provider.id) && (
                    <span className={`badge ${hasKey ? 'api-key-mode' : 'free-mode'}`}>
                      {currentMode}
                    </span>
                  )}
                  {provider.supports_websocket && <span className="badge ws-support">WebSocket</span>}
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
                  {/* Mode Toggle: Free vs API Key */}
                  {showModeToggle(provider.id) && (
                    <div className="form-group">
                      <label>使用模式</label>
                      <div className="mode-toggle">
                        {canUseFreeMode(provider.id) && (
                          <button
                            type="button"
                            className={`mode-btn ${!useKeyMode ? 'active' : ''}`}
                            onClick={() => handleModeSwitch(false)}
                          >
                            免費版
                            {info && <span className="mode-interval">{info.free_interval / 1000}秒</span>}
                          </button>
                        )}
                        <button
                          type="button"
                          className={`mode-btn ${useKeyMode ? 'active' : ''}`}
                          onClick={() => handleModeSwitch(true)}
                        >
                          API Key 版
                          {info && <span className="mode-interval">{info.key_interval / 1000}秒</span>}
                        </button>
                      </div>
                    </div>
                  )}

                  {/* API Key input - show when in key mode or when provider requires key */}
                  {useKeyMode && (info?.requires_api_key || info?.optional_api_key) && (
                    <div className="form-group">
                      <label>
                        API Key
                        {info?.optional_api_key && !info?.requires_api_key && (
                          <span className="optional-badge">提高速率限制</span>
                        )}
                      </label>
                      <input
                        type="password"
                        value={formData.api_key || ''}
                        onChange={(e) => setFormData({ ...formData, api_key: e.target.value })}
                        placeholder="輸入 API Key"
                      />
                    </div>
                  )}
                  {useKeyMode && info?.requires_api_secret && (
                    <div className="form-group">
                      <label>API Secret</label>
                      <input
                        type="password"
                        value={formData.api_secret || ''}
                        onChange={(e) => setFormData({ ...formData, api_secret: e.target.value })}
                        placeholder="輸入 API Secret"
                      />
                    </div>
                  )}
                  <div className="form-group">
                    <label>
                      刷新間隔 (毫秒)
                      {info && (
                        <span className="optional-badge">
                          建議: {(useKeyMode ? info.key_interval : info.free_interval) / 1000}秒
                        </span>
                      )}
                    </label>
                    <input
                      type="number"
                      value={formData.refresh_interval || 30000}
                      onChange={(e) => setFormData({ ...formData, refresh_interval: parseInt(e.target.value) })}
                      min={5000}
                      step={1000}
                    />
                  </div>
                  {provider.supports_websocket && (
                    <div className="form-group">
                      <label>連接方式</label>
                      <select
                        value={formData.connection_type || 'rest'}
                        onChange={(e) => setFormData({ ...formData, connection_type: e.target.value })}
                      >
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
                  <span>刷新: {provider.refresh_interval / 1000}秒</span>
                  <span>連接: {provider.connection_type === 'websocket' ? 'WebSocket' : 'REST'}</span>
                  {provider.api_key && <span className="api-status">🔑 已設定</span>}
                  <button className="btn-edit" onClick={() => handleEdit(provider)}>編輯</button>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
