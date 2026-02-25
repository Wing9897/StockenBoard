import { useState, useRef, useEffect, useMemo, memo, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Subscription, ProviderInfo } from '../../types';
import { useAssetPrice } from '../../hooks/useAssetData';
import { CountdownCircle } from './CountdownCircle';
import { AssetIcon, getIconName, invalidateIcon } from './AssetIcon';
import { formatPrice, formatNumber, summarizeError } from '../../lib/format';
import { t } from '../../lib/i18n';
import './AssetCard.css';

interface AssetCardProps {
  subscription: Subscription;
  providers: ProviderInfo[];
  currentProviderId: string;
  assetType: 'crypto' | 'stock';
  refreshInterval: number;
  onRemove: (id: number) => void;
  onEdit: (id: number, updates: { symbol?: string; displayName?: string; providerId?: string; assetType?: 'crypto' | 'stock' }) => Promise<void>;
  viewMode?: 'grid' | 'list' | 'compact';
  isCustomView?: boolean;
}

function formatExtraKey(key: string): string {
  const label = (t.extraFields as Record<string, string>)[key];
  if (label) return label;
  return key.replace(/_/g, ' ').replace(/([A-Z])/g, ' $1').replace(/^./, s => s.toUpperCase()).trim();
}

function formatExtraValue(value: unknown): string {
  if (value === null || value === undefined) return '-';
  if (typeof value === 'number') {
    if (Math.abs(value) >= 1e6) return formatNumber(value);
    if (Number.isInteger(value)) return value.toLocaleString();
    return value.toFixed(4);
  }
  if (typeof value === 'boolean') return value ? t.common.yes : t.common.no;
  if (typeof value === 'string') return value;
  return JSON.stringify(value);
}

export const AssetCard = memo(function AssetCard({ subscription, providers, currentProviderId, assetType, refreshInterval, onRemove, onEdit, viewMode = 'grid', isCustomView = false }: AssetCardProps) {
  const { asset, error } = useAssetPrice(subscription.symbol, currentProviderId);
  const [expanded, setExpanded] = useState(false);
  const [errorExpanded, setErrorExpanded] = useState(false);
  const [editing, setEditing] = useState(false);
  const [iconKey, setIconKey] = useState(0);
  const iconName = getIconName(subscription.symbol);

  const [editSymbol, setEditSymbol] = useState('');
  const [editDisplayName, setEditDisplayName] = useState('');
  const [editProvider, setEditProvider] = useState('');
  const [editAssetType, setEditAssetType] = useState<'crypto' | 'stock'>('crypto');
  const [editError, setEditError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const editRef = useRef<HTMLDivElement>(null);

  const handleIconClick = useCallback(async () => {
    try {
      await invoke('set_icon', { symbol: subscription.symbol });
      invalidateIcon(iconName);
      setIconKey(v => v + 1);
    } catch { /* cancelled */ }
  }, [subscription.symbol, iconName]);

  const changePercent = asset?.change_percent_24h ?? 0;
  const isPositive = changePercent >= 0;
  const currentProvider = providers.find(p => p.id === currentProviderId);

  useEffect(() => {
    if (error) {
      console.warn(`[${subscription.symbol}@${currentProviderId}]`, error);
      setErrorExpanded(false);
    }
  }, [error, subscription.symbol, currentProviderId]);

  const filteredProviders = useMemo(() => providers.filter(p =>
    editing
      ? (editAssetType === 'crypto' ? (p.provider_type === 'crypto' || p.provider_type === 'both' || p.provider_type === 'dex') : (p.provider_type === 'stock' || p.provider_type === 'both'))
      : (assetType === 'crypto' ? (p.provider_type === 'crypto' || p.provider_type === 'both' || p.provider_type === 'dex') : (p.provider_type === 'stock' || p.provider_type === 'both'))
  ), [providers, editing, editAssetType, assetType]);

  const editProviderInfo = providers.find(p => p.id === editProvider);
  const isEditDex = editProviderInfo?.provider_type === 'dex';

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
    if (!sym) { setEditError(t.subForm.symbolEmpty); return; }

    setSaving(true);
    setEditError(null);

    if (sym.toUpperCase() !== subscription.symbol.toUpperCase()) {
      try {
        await invoke('fetch_asset_price', { providerId: editProvider, symbol: sym });
      } catch (err) {
        setEditError(t.subForm.validateFailed(sym, err instanceof Error ? err.message : String(err)));
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
      setEditError(t.dex.saveFailed(err instanceof Error ? err.message : String(err)));
    } finally {
      setSaving(false);
    }
  };

  useEffect(() => {
    if (!editing) return;
    const handler = (e: MouseEvent) => {
      if (editRef.current && !editRef.current.contains(e.target as Node)) cancelEdit();
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [editing]);

  const renderIcon = (className: string) => (
    <AssetIcon key={iconKey} symbol={subscription.symbol} className={className} onClick={handleIconClick} />
  );

  const editPanel = (
    <div className="asset-edit-panel" ref={editRef}>
      <div className="edit-row">
        <label>{isEditDex ? t.subForm.symbolDex : t.subForm.symbol}</label>
        <input value={editSymbol} onChange={e => { setEditSymbol(e.target.value); setEditError(null); }} disabled={saving}
          placeholder={isEditDex ? (editProvider === 'jupiter' ? t.dex.solMintEditPlaceholder : t.dex.ethAddrEditPlaceholder) : ''}
          className={isEditDex ? 'dex-address-input' : undefined}
        />
        {isEditDex && editProvider === 'jupiter' && <span className="edit-hint">{t.subForm.jupiterEditHint}</span>}
        {isEditDex && editProvider === 'okx_dex' && <span className="edit-hint">{t.subForm.okxDexEditHint}</span>}
      </div>
      <div className="edit-row">
        <label>{t.dex.nickname}</label>
        <input value={editDisplayName} onChange={e => setEditDisplayName(e.target.value)} placeholder={t.dex.nicknameOptional} disabled={saving} />
      </div>
      <div className="edit-row">
        <label>{t.common.type}</label>
        <div className="edit-type-toggle">
          <button type="button" className={editAssetType === 'crypto' ? 'active' : ''} onClick={() => { setEditAssetType('crypto'); setEditProvider('binance'); }} disabled={saving}>{t.subForm.cryptoShort}</button>
          <button type="button" className={editAssetType === 'stock' ? 'active' : ''} onClick={() => { setEditAssetType('stock'); setEditProvider('yahoo'); }} disabled={saving}>{t.subForm.stockShort}</button>
        </div>
      </div>
      <div className="edit-row">
        <label>{t.dex.provider}</label>
        <select value={editProvider} onChange={e => setEditProvider(e.target.value)} disabled={saving}>
          {filteredProviders.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}
        </select>
        {editProviderInfo?.requires_api_key && <span className="edit-hint" style={{ color: 'var(--yellow)' }}>{t.subForm.apiKeyRequired}</span>}
      </div>
      {editError && <div className="edit-error">{editError}</div>}
      <div className="edit-actions">
        <button className="edit-btn delete" onClick={() => { onRemove(subscription.id); setEditing(false); }}>{isCustomView ? t.subs.removeDisplay : t.common.delete}</button>
        <div className="edit-actions-right">
          <button className="edit-btn cancel" onClick={cancelEdit} disabled={saving}>{t.common.cancel}</button>
          <button className="edit-btn save" onClick={saveEdit} disabled={saving}>{saving ? t.common.saving : t.common.save}</button>
        </div>
      </div>
    </div>
  );

  if (viewMode === 'compact') {
    return (
      <div className="asset-card-compact">
        <div className="compact-top">
          {renderIcon('asset-icon compact-icon')}
          <span className="compact-symbol" title={subscription.symbol}>{subscription.symbol}</span>
          <span className={`asset-type-tag ${assetType}`}>{assetType === 'crypto' ? t.subForm.cryptoShort : t.subForm.stockShort}</span>
          <button className="asset-card-edit-btn" onClick={openEdit} title={t.common.edit}>✎</button>
        </div>
        <div className="compact-bottom">
          <span className="compact-price">
            {error ? <span className="asset-error" title={summarizeError(error)}>{t.common.error}</span> : asset ? formatPrice(asset.price, asset.currency) : '-'}
          </span>
          {asset && !error && (
            <span className={`compact-change ${isPositive ? 'positive' : 'negative'}`}>
              {isPositive ? '▲' : '▼'} {Math.abs(changePercent).toFixed(2)}%
            </span>
          )}
          {refreshInterval > 0 && <CountdownCircle providerId={currentProviderId} fallbackInterval={refreshInterval} size={16} />}
        </div>
        {editing && editPanel}
      </div>
    );
  }

  if (viewMode === 'list') {
    return (
      <div className="asset-card-list">
        {renderIcon('asset-list-icon')}
        <div className="asset-list-symbol">
          <span className="symbol" title={subscription.symbol}>{subscription.symbol} <span className={`asset-type-tag ${assetType}`}>{assetType === 'crypto' ? t.subForm.cryptoShort : t.subForm.stockShort}</span></span>
          {subscription.display_name && <span className="name" title={subscription.display_name}>{subscription.display_name}</span>}
        </div>
        <div className="asset-list-price">
          {error ? <span className="asset-error" title={summarizeError(error)}>{t.common.error}</span> : asset ? formatPrice(asset.price, asset.currency) : t.common.loading}
        </div>
        <div className={`asset-list-change ${isPositive ? 'positive' : 'negative'}`}>
          {asset && !error && <>{isPositive ? '▲' : '▼'} {Math.abs(changePercent).toFixed(2)}%</>}
        </div>
        <span className="asset-list-provider-label">{t.dex.dataSource(currentProvider?.name || currentProviderId)}</span>
        <button className="asset-card-edit-btn" onClick={openEdit} title={t.common.edit}>✎</button>
        {refreshInterval > 0 && <CountdownCircle providerId={currentProviderId} fallbackInterval={refreshInterval} size={22} />}
        {editing && editPanel}
      </div>
    );
  }

  return (
    <div className="asset-card">
      <div className="asset-card-header">
        {renderIcon('asset-icon')}
        <div className="asset-info">
          <p className="asset-symbol"><span className="asset-symbol-text" title={subscription.symbol}>{subscription.symbol}</span> <span className={`asset-type-tag ${assetType}`}>{assetType === 'crypto' ? t.subForm.cryptoShort : t.subForm.stockShort}</span></p>
          <p className="asset-name" title={subscription.display_name || ''}>{subscription.display_name || ''}</p>
        </div>
        <button className="asset-card-edit-btn" onClick={openEdit} title={t.common.edit}>✎</button>
        {refreshInterval > 0 && <CountdownCircle providerId={currentProviderId} fallbackInterval={refreshInterval} size={20} />}
      </div>

      <div className="asset-card-body">
        <p className="asset-price">
          {error ? <span className="asset-error">{t.dex.fetchFailed}</span> : asset ? formatPrice(asset.price, asset.currency) : t.common.loading}
        </p>
        {asset && !error && (
          <span className={`asset-change ${isPositive ? 'positive' : 'negative'}`}>
            {isPositive ? '▲' : '▼'} {Math.abs(changePercent).toFixed(2)}%
          </span>
        )}
      </div>

      {error && (
        <div className="asset-error-detail" onClick={() => setErrorExpanded(v => !v)} title={t.asset.clickExpandCollapse}>
          <span className="asset-error-summary">{summarizeError(error)}</span>
          {errorExpanded && <pre className="asset-error-full">{error}</pre>}
        </div>
      )}

      {asset && !error && (
        <div className="asset-card-stats">
          {asset.high_24h !== undefined && (
            <div className="asset-stat"><span className="asset-stat-label">{t.asset.high24h}</span><span className="asset-stat-value">{formatPrice(asset.high_24h, asset.currency)}</span></div>
          )}
          {asset.low_24h !== undefined && (
            <div className="asset-stat"><span className="asset-stat-label">{t.asset.low24h}</span><span className="asset-stat-value">{formatPrice(asset.low_24h, asset.currency)}</span></div>
          )}
          {asset.volume !== undefined && (
            <div className="asset-stat"><span className="asset-stat-label">{t.asset.volume}</span><span className="asset-stat-value">{formatNumber(asset.volume)}</span></div>
          )}
        </div>
      )}

      <div className="asset-card-footer">
        <span className="asset-footer-provider">{t.dex.dataSource(currentProvider?.name || currentProviderId)}</span>
      </div>

      <button className="asset-card-toggle" onClick={() => setExpanded(!expanded)}>
        {expanded ? t.asset.collapse : t.asset.expand}
      </button>

      {expanded && asset && (
        <div className="asset-card-expanded">
          <div className="asset-card-extra">
            {asset.market_cap !== undefined && (
              <div className="asset-stat"><span className="asset-stat-label">{t.asset.marketCap}</span><span className="asset-stat-value">{formatNumber(asset.market_cap)}</span></div>
            )}
            {asset.change_24h !== undefined && (
              <div className="asset-stat"><span className="asset-stat-label">{t.asset.change24h}</span><span className="asset-stat-value">{formatPrice(asset.change_24h, asset.currency)}</span></div>
            )}
            {asset.extra && Object.entries(asset.extra).map(([key, value]) => (
              <div className="asset-stat" key={key}><span className="asset-stat-label">{formatExtraKey(key)}</span><span className="asset-stat-value">{formatExtraValue(value)}</span></div>
            ))}
            <div className="asset-stat"><span className="asset-stat-label">{t.asset.updatedAt}</span><span className="asset-stat-value">{new Date(asset.last_updated).toLocaleTimeString()}</span></div>
          </div>
        </div>
      )}

      {editing && editPanel}
    </div>
  );
});
