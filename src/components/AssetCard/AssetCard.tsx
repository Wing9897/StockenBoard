import { useState, useRef, useEffect, memo } from 'react';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
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
  viewMode?: 'grid' | 'list' | 'compact';
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

// Cache icons dir path — resolved once, reused by all cards
let _iconsDirCache: string | null = null;
let _iconsDirPromise: Promise<string> | null = null;
function getIconsDir(): Promise<string> {
  if (_iconsDirCache) return Promise.resolve(_iconsDirCache);
  if (!_iconsDirPromise) {
    _iconsDirPromise = invoke<string>('get_icons_dir').then(dir => {
      _iconsDirCache = dir;
      return dir;
    });
  }
  return _iconsDirPromise;
}

// 記住哪些 icon 確認不存在，避免重複嘗試載入 404
const _missingIcons = new Set<string>();

function getIconName(symbol: string): string {
  return symbol.toLowerCase().replace(/usdt$/, '').replace(/-usd$/, '');
}

export const AssetCard = memo(function AssetCard({ asset, error, subscription, providers, currentProviderId, assetType, refreshTiming, onRemove, onEdit, viewMode = 'grid' }: AssetCardProps) {
  const [expanded, setExpanded] = useState(false);
  const [iconError, setIconError] = useState(false);
  const [editing, setEditing] = useState(false);
  const [customIconSrc, setCustomIconSrc] = useState<string | null>(null);
  const [iconVersion, setIconVersion] = useState(0);
  const iconName = getIconName(subscription.symbol);

  // Edit form state
  const [editSymbol, setEditSymbol] = useState('');
  const [editDisplayName, setEditDisplayName] = useState('');
  const [editProvider, setEditProvider] = useState('');
  const [editAssetType, setEditAssetType] = useState<'crypto' | 'stock'>('crypto');
  const [editError, setEditError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const editRef = useRef<HTMLDivElement>(null);

  // Load custom icon from app data dir
  useEffect(() => {
    // 已知不存在的 icon 直接跳過
    if (_missingIcons.has(iconName)) return;

    let cancelled = false;
    getIconsDir().then(dir => {
      if (cancelled) return;
      const sep = dir.endsWith('\\') || dir.endsWith('/') ? '' : '/';
      setCustomIconSrc(convertFileSrc(`${dir}${sep}${iconName}.png`));
    }).catch(() => {
      if (!cancelled) setCustomIconSrc(null);
    });
    return () => { cancelled = true; };
  }, [iconName, iconVersion]);

  const handleIconClick = async () => {
    try {
      await invoke('set_icon', { symbol: subscription.symbol });
      _missingIcons.delete(iconName);
      setIconError(false);
      setCustomIconSrc(null);
      setIconVersion(v => v + 1);
    } catch {
      // user cancelled — do nothing
    }
  };

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
    if (customIconSrc && !iconError) return customIconSrc;
    if (_missingIcons.has(iconName)) return null;
    return `/icons/${iconName}.png`;
  };

  const handleIconError = () => {
    if (customIconSrc && !iconError) {
      // Custom icon failed → 記住此 icon 不存在，嘗試 public fallback
      _missingIcons.add(iconName);
      setCustomIconSrc(null);
    } else {
      setIconError(true);
    }
  };

  const iconSrc = getIconSrc();
  const iconFallbackText = iconName.slice(0, 3).toUpperCase();

  const renderIcon = (className: string) => (
    <div className={`${className} clickable`} onClick={handleIconClick} title="點擊設定圖示">
      {iconSrc && !iconError ? (
        <img src={iconSrc} alt={subscription.symbol} onError={handleIconError} />
      ) : (
        <span className="asset-icon-fallback">{iconFallbackText}</span>
      )}
    </div>
  );

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

  // Compact view — mini card
  if (viewMode === 'compact') {
    return (
      <div className="asset-card-compact">
        <div className="compact-top">
          {renderIcon('asset-icon compact-icon')}
          <span className="compact-symbol">{subscription.symbol}</span>
          <span className={`asset-type-tag ${assetType}`}>{assetType === 'crypto' ? '幣' : '股'}</span>
          <button className="asset-card-edit-btn" onClick={openEdit} title="編輯">✎</button>
        </div>
        <div className="compact-bottom">
          <span className="compact-price">
            {error ? <span className="asset-error">錯誤</span> : asset ? formatPrice(asset.price, asset.currency) : '-'}
          </span>
          {asset && !error && (
            <span className={`compact-change ${isPositive ? 'positive' : 'negative'}`}>
              {isPositive ? '▲' : '▼'} {Math.abs(changePercent).toFixed(2)}%
            </span>
          )}
          {refreshTiming && <CountdownCircle interval={refreshTiming.interval} lastFetch={refreshTiming.lastFetch} size={16} />}
        </div>
        {editing && editPanel}
      </div>
    );
  }

  // List view
  if (viewMode === 'list') {
    return (
      <div className="asset-card-list">
        {renderIcon('asset-list-icon')}
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
        {renderIcon('asset-icon')}
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
});
