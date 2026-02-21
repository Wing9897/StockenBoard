import { useState, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AssetData, Subscription, ProviderInfo } from '../../types';
import { CountdownCircle } from './CountdownCircle';
import './AssetCard.css';

interface AssetCardProps {
  asset: AssetData | undefined;
  error?: string;
  subscription: Subscription;
  providers: ProviderInfo[];
  currentProviderId: string;
  assetType: 'crypto' | 'stock';
  refreshTiming?: { interval: number; lastFetch: number };
  onRemove: (id: number) => void;
  onEdit: (id: number, updates: { symbol?: string; displayName?: string; providerId?: string; assetType?: 'crypto' | 'stock' }) => Promise<void>;
  viewMode?: 'grid' | 'list';
}

function formatNumber(num: number | undefined, decimals = 2): string {
  if (num === undefined || num === null) return '-';
  if (num >= 1e12) return (num / 1e12).toFixed(2) + 'T';
  if (num >= 1e9) return (num / 1e9).toFixed(2) + 'B';
  if (num >= 1e6) return (num / 1e6).toFixed(2) + 'M';
  if (num >= 1e3) return (num / 1e3).toFixed(2) + 'K';
  return num.toFixed(decimals);
}

function formatPrice(price: number | undefined | null, currency: string = 'USD'): string {
  if (price === undefined || price === null || isNaN(price)) return '-';
  const sym = currency === 'USD' || currency === 'USDT' ? '$' : currency + ' ';
  if (price >= 1) return sym + price.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  return sym + price.toPrecision(4);
}

function formatExtraKey(key: string): string {
  if (/[\u4e00-\u9fa5]/.test(key)) return key;
  return key.replace(/_/g, ' ').replace(/([A-Z])/g, ' $1').replace(/^./, s => s.toUpperCase()).trim();
}

function formatExtraValue(value: unknown): string {
  if (value === null || value === undefined) return '-';
  if (typeof value === 'number') {
    if (Math.abs(value) >= 1e6) return formatNumber(value);
    if (Number.isInteger(value)) return value.toLocaleString();
    return value.toFixed(4);
  }
  if (typeof value === 'boolean') return value ? '是' : '否';
  if (typeof value === 'string') return value;
  return JSON.stringify(value);
}

export function AssetCard({ asset, error, subscription, providers, currentProviderId, assetType, refreshTiming, onRemove, onEdit, viewMode = 'grid' }: AssetCardProps) {
  const [expanded, setExpanded] = useState(false);
  const [iconError, setIconError] = useState(false);
  const [editing, setEditing] = useState(false);

  // Edit form state
  const [editSymbol, setEditSymbol] = useState('');
  const [editDisplayName, setEditDisplayName] = useState('');
  const [editProvider, setEditProvider] = useState('');
  const [editAssetType, setEditAssetType] = useState<'crypto' | 'stock'>('crypto');
  const [editError, setEditError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const editRef = useRef<HTMLDivElement>(null);

  const changePercent = asset?.change_percent_24h ?? 0;
  const isPositive = changePercent >= 0;
  const currentProvider = providers.find(p => p.id === currentProviderId);

  const filteredProviders = providers.filter(p =>
    editing
      ? (editAssetType === 'crypto' ? (p.provider_type === 'crypto' || p.provider_type === 'both') : (p.provider_type === 'stock' || p.provider_type === 'both'))
      : (assetType === 'crypto' ? (p.provider_type === 'crypto' || p.provider_type === 'both') : (p.provider_type === 'stock' || p.provider_type === 'both'))
  );

  const openEdit = () => {
    setEditSymbol(subscription.symbol);
    setEditDisplayName(subscription.display_name || '');
    setEditProvider(currentProviderId);
    setEditAssetType(assetType);
    setEditError(null);
    setEditing(true);
  };

  const cancelEdit = () => { setEditing(false); setEditError(null); };

  const saveEdit = async () => {
    const sym = editSymbol.trim();
    if (!sym) { setEditError('代號不能為空'); return; }

    setSaving(true);
    setEditError(null);

    // Validate symbol if changed
    if (sym.toUpperCase() !== subscription.symbol.toUpperCase()) {
      try {
        await invoke('fetch_asset_price', { providerId: editProvider, symbol: sym });
      } catch (err) {
        setEditError(`無法驗證 "${sym}": ${err instanceof Error ? err.message : String(err)}`);
        setSaving(false);
        return;
      }
    }

    try {
      await onEdit(subscription.id, {
        symbol: sym,
        displayName: editDisplayName,
        providerId: editProvider,
        assetType: editAssetType,
      });
      setEditing(false);
    } catch (err) {
      setEditError(`儲存失敗: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setSaving(false);
    }
  };

  // Close edit panel on outside click
  useEffect(() => {
    if (!editing) return;
    const handler = (e: MouseEvent) => {
      if (editRef.current && !editRef.current.contains(e.target as Node)) cancelEdit();
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [editing]);

  const getIconSrc = () => {
    const symbolLower = subscription.symbol.toLowerCase().replace(/usdt$/, '').replace(/-usd$/, '');
    return `/icons/${symbolLower}.png`;
  };

  // Edit panel (shared between grid and list)
  const editPanel = (
    <div className="asset-edit-panel" ref={editRef}>
      <div className="edit-row">
        <label>代號</label>
        <input value={editSymbol} onChange={e => { setEditSymbol(e.target.value); setEditError(null); }} disabled={saving} />
      </div>
      <div className="edit-row">
        <label>暱稱</label>
        <input value={editDisplayName} onChange={e => setEditDisplayName(e.target.value)} placeholder="可選" disabled={saving} />
      </div>
      <div className="edit-row">
        <label>類型</label>
        <div className="edit-type-toggle">
          <button type="button" className={editAssetType === 'crypto' ? 'active' : ''} onClick={() => { setEditAssetType('crypto'); setEditProvider('binance'); }} disabled={saving}>幣</button>
          <button type="button" className={editAssetType === 'stock' ? 'active' : ''} onClick={() => { setEditAssetType('stock'); setEditProvider('yahoo'); }} disabled={saving}>股</button>
        </div>
      </div>
      <div className="edit-row">
        <label>數據源</label>
        <select value={editProvider} onChange={e => setEditProvider(e.target.value)} disabled={saving}>
          {filteredProviders.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}
        </select>
      </div>
      {editError && <div className="edit-error">{editError}</div>}
      <div className="edit-actions">
        <button className="edit-btn delete" onClick={() => { onRemove(subscription.id); setEditing(false); }}>刪除</button>
        <div className="edit-actions-right">
          <button className="edit-btn cancel" onClick={cancelEdit} disabled={saving}>取消</button>
          <button className="edit-btn save" onClick={saveEdit} disabled={saving}>{saving ? '儲存中...' : '儲存'}</button>
        </div>
      </div>
    </div>
  );

  // List view
  if (viewMode === 'list') {
    return (
      <div className="asset-card-list">
        <div className="asset-list-icon">
          {!iconError ? (
            <img src={getIconSrc()} alt={subscription.symbol} onError={() => setIconError(true)} />
          ) : (
            <span className="asset-icon-fallback">{subscription.symbol.replace(/USDT$/, '').replace(/-USD$/, '').slice(0, 3)}</span>
          )}
        </div>
        <div className="asset-list-symbol">
          <span className="symbol">{subscription.symbol} <span className={`asset-type-tag ${assetType}`}>{assetType === 'crypto' ? '幣' : '股'}</span></span>
          {subscription.display_name && <span className="name">{subscription.display_name}</span>}
        </div>
        <div className="asset-list-price">
          {error ? <span className="asset-error">錯誤</span> : asset ? formatPrice(asset.price, asset.currency) : '載入中...'}
        </div>
        <div className={`asset-list-change ${isPositive ? 'positive' : 'negative'}`}>
          {asset && !error && <>{isPositive ? '▲' : '▼'} {Math.abs(changePercent).toFixed(2)}%</>}
        </div>
        <span className="asset-list-provider-label">數據源: {currentProvider?.name || currentProviderId}</span>
        <button className="asset-card-edit-btn" onClick={openEdit} title="編輯">✎</button>
        {refreshTiming && <CountdownCircle interval={refreshTiming.interval} lastFetch={refreshTiming.lastFetch} size={22} />}
        {editing && editPanel}
      </div>
    );
  }

  // Grid view (default)
  return (
    <div className="asset-card">
      <div className="asset-card-header">
        <div className="asset-icon">
          {!iconError ? (
            <img src={getIconSrc()} alt={subscription.symbol} onError={() => setIconError(true)} />
          ) : (
            <span className="asset-icon-fallback">{subscription.symbol.replace(/USDT$/, '').replace(/-USD$/, '').slice(0, 3)}</span>
          )}
        </div>
        <div className="asset-info">
          <p className="asset-symbol">{subscription.symbol} <span className={`asset-type-tag ${assetType}`}>{assetType === 'crypto' ? '幣' : '股'}</span></p>
          <p className="asset-name">{subscription.display_name || ''}</p>
        </div>
        <button className="asset-card-edit-btn" onClick={openEdit} title="編輯">✎</button>
        {refreshTiming && <CountdownCircle interval={refreshTiming.interval} lastFetch={refreshTiming.lastFetch} size={20} />}
      </div>

      <div className="asset-card-body">
        <p className="asset-price">
          {error ? <span className="asset-error">獲取失敗</span> : asset ? formatPrice(asset.price, asset.currency) : '載入中...'}
        </p>
        {asset && !error && (
          <span className={`asset-change ${isPositive ? 'positive' : 'negative'}`}>
            {isPositive ? '▲' : '▼'} {Math.abs(changePercent).toFixed(2)}%
          </span>
        )}
      </div>

      {error && <div className="asset-error-detail">{error}</div>}

      {asset && !error && (
        <div className="asset-card-stats">
          {asset.high_24h !== undefined && (
            <div className="asset-stat"><span className="asset-stat-label">24H 高</span><span className="asset-stat-value">{formatPrice(asset.high_24h, asset.currency)}</span></div>
          )}
          {asset.low_24h !== undefined && (
            <div className="asset-stat"><span className="asset-stat-label">24H 低</span><span className="asset-stat-value">{formatPrice(asset.low_24h, asset.currency)}</span></div>
          )}
          {asset.volume !== undefined && (
            <div className="asset-stat"><span className="asset-stat-label">成交量</span><span className="asset-stat-value">{formatNumber(asset.volume)}</span></div>
          )}
        </div>
      )}

      <div className="asset-card-footer">
        <span className="asset-footer-provider">數據源: {currentProvider?.name || currentProviderId}</span>
      </div>

      <button className="asset-card-toggle" onClick={() => setExpanded(!expanded)}>
        {expanded ? '▲ 收起' : '▼ 顯示更多'}
      </button>

      {expanded && asset && (
        <div className="asset-card-expanded">
          <div className="asset-card-extra">
            {asset.market_cap !== undefined && (
              <div className="asset-stat"><span className="asset-stat-label">市值</span><span className="asset-stat-value">{formatNumber(asset.market_cap)}</span></div>
            )}
            {asset.change_24h !== undefined && (
              <div className="asset-stat"><span className="asset-stat-label">24H 變動</span><span className="asset-stat-value">{formatPrice(asset.change_24h, asset.currency)}</span></div>
            )}
            {asset.extra && Object.entries(asset.extra).map(([key, value]) => (
              <div className="asset-stat" key={key}><span className="asset-stat-label">{formatExtraKey(key)}</span><span className="asset-stat-value">{formatExtraValue(value)}</span></div>
            ))}
            <div className="asset-stat"><span className="asset-stat-label">更新時間</span><span className="asset-stat-value">{new Date(asset.last_updated).toLocaleTimeString()}</span></div>
          </div>
        </div>
      )}

      {editing && editPanel}
    </div>
  );
}
