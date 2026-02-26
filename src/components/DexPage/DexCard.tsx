import { useState, useRef, useEffect, memo, useCallback, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Subscription, ProviderInfo } from '../../types';
import { useAssetPrice } from '../../hooks/useAssetData';
import { CountdownCircle } from '../AssetCard/CountdownCircle';
import { AssetIcon, getIconName, invalidateIcon } from '../AssetCard/AssetIcon';
import { formatPrice, formatNumber, summarizeError } from '../../lib/format';
import { t } from '../../lib/i18n';
import './DexCard.css';

interface DexCardProps {
  subscription: Subscription;
  providers: ProviderInfo[];
  refreshInterval: number;
  onRemove: (id: number) => void;
  onEdit: (id: number, updates: {
    poolAddress?: string; tokenFrom?: string; tokenTo?: string;
    providerId?: string; displayName?: string;
  }) => Promise<void>;
  viewMode: 'grid' | 'list' | 'compact';
  isCustomView?: boolean;
  getDexSymbol: (sub: Subscription) => string;
}

function truncateAddr(addr: string, len = 6): string {
  if (!addr) return '-';
  if (addr.length <= len * 2 + 2) return addr;
  return `${addr.slice(0, len)}...${addr.slice(-4)}`;
}

function parsePairFromName(displayName: string | undefined): [string, string] {
  const dn = displayName || '';
  const sep = dn.includes('/') ? '/' : dn.includes('→') ? '→' : null;
  if (sep) {
    const parts = dn.split(sep).map(s => s.trim());
    if (parts.length === 2 && parts[0] && parts[1]) return [parts[0], parts[1]];
  }
  return ['', ''];
}

export const DexCard = memo(function DexCard({
  subscription, providers, refreshInterval, onRemove, onEdit, viewMode, isCustomView = false, getDexSymbol,
}: DexCardProps) {
  const symbol = getDexSymbol(subscription);
  const { asset, error } = useAssetPrice(symbol, subscription.selected_provider_id);
  const [editing, setEditing] = useState(false);
  const [errorExpanded, setErrorExpanded] = useState(false);

  const poolAddress = subscription.pool_address || '';
  const tokenFrom = subscription.token_from_address || '';
  const tokenTo = subscription.token_to_address || '';

  const [editPool, setEditPool] = useState('');
  const [editTokenFrom, setEditTokenFrom] = useState('');
  const [editTokenTo, setEditTokenTo] = useState('');
  const [editFromSymbol, setEditFromSymbol] = useState('');
  const [editToSymbol, setEditToSymbol] = useState('');
  const [editProvider, setEditProvider] = useState('');
  const [editDisplayName, setEditDisplayName] = useState('');
  const [editError, setEditError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [lookingUp, setLookingUp] = useState(false);
  const [editManualTokens, setEditManualTokens] = useState(false);
  const editRef = useRef<HTMLDivElement>(null);

  const extra = asset?.extra as Record<string, unknown> | undefined;
  const gasEstimate = extra?.gas_estimate as string | undefined;
  const routePath = extra?.route_path as string | undefined;
  const poolTvl = extra?.pool_tvl as number | undefined;
  const amountOut = extra?.amount_out as number | undefined;

  const [fromIconSymbol, toIconSymbol] = parsePairFromName(subscription.display_name);
  const fromIconName = getIconName(fromIconSymbol);
  const toIconName = getIconName(toIconSymbol);
  const [iconKey, setIconKey] = useState(0);

  const handleFromIconClick = useCallback(async () => {
    if (!fromIconSymbol) return;
    try {
      await invoke('set_icon', { symbol: fromIconSymbol });
      invalidateIcon(fromIconName);
      setIconKey(v => v + 1);
    } catch { /* cancelled */ }
  }, [fromIconSymbol, fromIconName]);

  const handleToIconClick = useCallback(async () => {
    if (!toIconSymbol) return;
    try {
      await invoke('set_icon', { symbol: toIconSymbol });
      invalidateIcon(toIconName);
      setIconKey(v => v + 1);
    } catch { /* cancelled */ }
  }, [toIconSymbol, toIconName]);

  const renderPairIcons = (className: string) => (
    <div className={`dex-pair-icons ${className}`}>
      {fromIconSymbol ? (
        <AssetIcon key={`from-${iconKey}`} symbol={fromIconSymbol} className="asset-icon dex-icon" onClick={handleFromIconClick} />
      ) : (
        <div className="asset-icon dex-icon"><span className="asset-icon-fallback">?</span></div>
      )}
      {toIconSymbol ? (
        <AssetIcon key={`to-${iconKey}`} symbol={toIconSymbol} className="asset-icon dex-icon" onClick={handleToIconClick} />
      ) : (
        <div className="asset-icon dex-icon"><span className="asset-icon-fallback">?</span></div>
      )}
    </div>
  );

  const providerName = providers.find(p => p.id === subscription.selected_provider_id)?.name
    || subscription.selected_provider_id;

  useEffect(() => {
    if (error) setErrorExpanded(false);
  }, [error]);

  const openEdit = useCallback(() => {
    const isJup = subscription.selected_provider_id === 'jupiter';
    setEditPool(isJup ? parsePairFromName(subscription.display_name).join(',') : poolAddress);
    setEditTokenFrom(tokenFrom);
    setEditTokenTo(tokenTo);
    setEditFromSymbol('');
    setEditToSymbol('');
    setEditProvider(subscription.selected_provider_id);
    setEditDisplayName(subscription.display_name || '');
    setEditError(null);
    setEditManualTokens(false);
    setEditing(true);
  }, [subscription, poolAddress, tokenFrom, tokenTo]);

  const cancelEdit = () => { setEditing(false); setEditError(null); };

  const handleEditLookup = async () => {
    const pool = editPool.trim();
    if (!pool) { setEditError(isEditJupiter ? t.errors.pairInputRequired : t.errors.poolInputRequired); return; }
    setLookingUp(true);
    setEditError(null);
    try {
      const info = await invoke<{ token0_address: string; token0_symbol: string; token1_address: string; token1_symbol: string }>(
        'lookup_dex_pool', { providerId: editProvider, poolAddress: pool }
      );
      setEditTokenFrom(info.token0_address);
      setEditTokenTo(info.token1_address);
      setEditFromSymbol(info.token0_symbol);
      setEditToSymbol(info.token1_symbol);
      if (editProvider === 'jupiter' && !editDisplayName) {
        setEditDisplayName(`${info.token0_symbol}/${info.token1_symbol}`);
      }
    } catch (err) {
      setEditError(t.dex.lookupFailed(err instanceof Error ? err.message : String(err)));
    } finally { setLookingUp(false); }
  };

  const handleEditSwap = () => {
    const tmpFrom = editTokenFrom;
    const tmpFromSym = editFromSymbol;
    setEditTokenFrom(editTokenTo);
    setEditTokenTo(tmpFrom);
    setEditFromSymbol(editToSymbol);
    setEditToSymbol(tmpFromSym);
  };

  const saveEdit = async () => {
    const isJup = editProvider === 'jupiter';
    const finalPool = isJup ? 'auto' : editPool.trim();
    if (!isJup && !finalPool) {
      setEditError(t.dex.poolEmpty); return;
    }
    if (!editTokenFrom.trim() || !editTokenTo.trim()) {
      setEditError(t.dex.tokenEmpty); return;
    }
    setSaving(true);
    setEditError(null);
    const testSymbol = `${finalPool}:${editTokenFrom.trim()}:${editTokenTo.trim()}`;
    try {
      await invoke('fetch_asset_price', { providerId: editProvider, symbol: testSymbol });
    } catch (err) {
      setEditError(t.dex.validateFailed(err instanceof Error ? err.message : String(err)));
      setSaving(false); return;
    }
    try {
      await onEdit(subscription.id, {
        poolAddress: finalPool, tokenFrom: editTokenFrom, tokenTo: editTokenTo,
        providerId: editProvider, displayName: editDisplayName,
      });
      setEditing(false);
    } catch (err) {
      setEditError(t.dex.saveFailed(err instanceof Error ? err.message : String(err)));
    } finally { setSaving(false); }
  };

  useEffect(() => {
    if (!editing) return;
    const handler = (e: MouseEvent) => {
      if (editRef.current && !editRef.current.contains(e.target as Node)) cancelEdit();
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [editing]);

  const editBusy = saving || lookingUp;
  const isEditJupiter = editProvider === 'jupiter';
  const dexProviders = useMemo(() => providers.filter(p => p.provider_type === 'dex'), [providers]);

  const editPanel = (
    <div className="dex-edit-panel" ref={editRef}>
      <div className="edit-row">
        <label>{t.dex.provider}</label>
        <select value={editProvider} onChange={e => setEditProvider(e.target.value)} disabled={editBusy}>
          {dexProviders.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}
        </select>
      </div>
      <div className="edit-row">
        <label>{isEditJupiter ? t.dex.tradePair : t.dex.poolAddress}</label>
        <div className="dex-edit-input-row">
          <input value={editPool} onChange={e => { setEditPool(e.target.value); setEditError(null); }} disabled={editBusy}
            placeholder={isEditJupiter ? t.dex.pairPlaceholder : editProvider === 'subgraph' ? t.dex.subgraphProtocolPlaceholder : t.dex.evmPoolPlaceholder}
            className="dex-address-input" />
          <button className="edit-btn save" onClick={handleEditLookup} disabled={editBusy}>
            {lookingUp ? t.dex.lookingUp : t.dex.lookup}
          </button>
        </div>
        {isEditJupiter && <span className="edit-hint">{t.dex.jupiterHint}</span>}
        {editProvider === 'subgraph' && <span className="edit-hint">{t.dex.subgraphHint}</span>}
      </div>
      <div className="edit-row">
        <label>{t.dex.tradeDirection}</label>
        {editManualTokens ? (
          <>
            <div className="dex-edit-token-col">
              <input value={editTokenFrom} onChange={e => { setEditTokenFrom(e.target.value); setEditError(null); }} disabled={editBusy}
                placeholder={t.dex.tokenFromPlaceholder} className="dex-address-input" />
              <div className="dex-edit-token-input-row">
                <input value={editTokenTo} onChange={e => { setEditTokenTo(e.target.value); setEditError(null); }} disabled={editBusy}
                  placeholder={t.dex.tokenToPlaceholder} className="dex-address-input" />
                <button className="edit-btn cancel dex-edit-swap-sm" onClick={handleEditSwap} disabled={editBusy} title={t.dex.flipDirection}>⇄</button>
              </div>
            </div>
            <button type="button" className="dex-edit-link-btn" onClick={() => setEditManualTokens(false)}>
              {t.dex.useAutoMode}
            </button>
          </>
        ) : (
          <>
            <div className="dex-edit-direction-row">
              <span className="dex-edit-direction-text">
                {editFromSymbol || truncateAddr(editTokenFrom)} → {editToSymbol || truncateAddr(editTokenTo)}
              </span>
              <button className="edit-btn cancel dex-edit-swap-sm" onClick={handleEditSwap} disabled={editBusy} title={t.dex.flipDirection}>⇄</button>
            </div>
            <button type="button" className="dex-edit-link-btn" onClick={() => setEditManualTokens(true)}>
              {t.dex.useManualMode}
            </button>
          </>
        )}
      </div>
      <div className="edit-row">
        <label>{t.dex.nickname}</label>
        <input value={editDisplayName} onChange={e => setEditDisplayName(e.target.value)} placeholder={t.dex.nicknameOptional} disabled={editBusy} />
      </div>
      {editError && <div className="edit-error">{editError}</div>}
      <div className="edit-actions">
        <button className="edit-btn delete" onClick={() => { onRemove(subscription.id); setEditing(false); }}>
          {isCustomView ? t.subs.removeDisplay : t.common.delete}
        </button>
        <div className="edit-actions-right">
          <button className="edit-btn cancel" onClick={cancelEdit} disabled={editBusy}>{t.common.cancel}</button>
          <button className="edit-btn save" onClick={saveEdit} disabled={editBusy}>{saving ? t.common.saving : t.common.save}</button>
        </div>
      </div>
    </div>
  );

  if (viewMode === 'compact') {
    return (
      <div className="dex-card-compact">
        <div className="compact-top">
          {renderPairIcons('compact-icons')}
          <span className="compact-symbol" title={`${tokenFrom} → ${tokenTo}`}>
            {subscription.display_name || `${truncateAddr(tokenFrom)}→${truncateAddr(tokenTo)}`}
          </span>
          <button className="asset-card-edit-btn" onClick={openEdit} title={t.common.edit}>✎</button>
        </div>
        <div className="compact-bottom">
          <span className="compact-price">
            {error ? <span className="asset-error" title={summarizeError(error)}>{t.common.error}</span> : asset ? formatPrice(asset.price) : '-'}
          </span>
          {refreshInterval > 0 && <CountdownCircle providerId={subscription.selected_provider_id} fallbackInterval={refreshInterval} size={16} />}
        </div>
        {editing && editPanel}
      </div>
    );
  }

  if (viewMode === 'list') {
    return (
      <div className="dex-card-list">
        {renderPairIcons('list-icons')}
        <div className="dex-list-symbol">
          <span className="symbol" title={`${tokenFrom} → ${tokenTo}`}>
            {subscription.display_name || `${truncateAddr(tokenFrom)} → ${truncateAddr(tokenTo)}`}
          </span>
          <span className="dex-pool-addr" title={poolAddress}>
            {subscription.selected_provider_id !== 'jupiter' ? t.dex.pool(truncateAddr(poolAddress)) : t.dex.jupiterAgg}
          </span>
        </div>
        <div className="dex-list-price">
          {error ? <span className="asset-error" title={summarizeError(error)}>{t.common.error}</span> : asset ? formatPrice(asset.price) : t.common.loading}
        </div>
        {amountOut !== undefined && (
          <div className="dex-list-swap">{t.dex.swapRateShort(amountOut.toPrecision(6))}</div>
        )}
        <span className="dex-list-provider">{t.dex.dataSource(providerName)}</span>
        <button className="asset-card-edit-btn" onClick={openEdit} title={t.common.edit}>✎</button>
        {refreshInterval > 0 && <CountdownCircle providerId={subscription.selected_provider_id} fallbackInterval={refreshInterval} size={22} />}
        {editing && editPanel}
      </div>
    );
  }

  return (
    <div className="dex-card">
      <div className="dex-card-header">
        {renderPairIcons('grid-icons')}
        <div className="dex-info">
          <p className="dex-pair" title={`${tokenFrom} → ${tokenTo}`}>
            {truncateAddr(tokenFrom)} → {truncateAddr(tokenTo)}
          </p>
          {subscription.display_name && <p className="dex-name">{subscription.display_name}</p>}
        </div>
        <button className="asset-card-edit-btn" onClick={openEdit} title={t.common.edit}>✎</button>
        {refreshInterval > 0 && <CountdownCircle providerId={subscription.selected_provider_id} fallbackInterval={refreshInterval} size={20} />}
      </div>

      <div className="dex-card-body">
        <p className="dex-price">
          {error ? <span className="asset-error">{t.dex.fetchFailed}</span> : asset ? formatPrice(asset.price) : t.common.loading}
        </p>
        {amountOut !== undefined && !error && (
          <p className="dex-swap-rate">{t.dex.swapRate(amountOut.toPrecision(6))}</p>
        )}
      </div>

      {error && (
        <div className="dex-error-detail" onClick={() => setErrorExpanded(v => !v)} title={t.dex.clickExpandCollapse}>
          <span className="dex-error-summary">{summarizeError(error)}</span>
          {errorExpanded && <pre className="dex-error-full">{error}</pre>}
        </div>
      )}

      {asset && !error && (
        <div className="dex-card-stats">
          {gasEstimate && (
            <div className="dex-stat"><span className="dex-stat-label">{t.dex.gasLabel}</span><span className="dex-stat-value">{gasEstimate}</span></div>
          )}
          {routePath && (
            <div className="dex-stat"><span className="dex-stat-label">{t.dex.routeLabel}</span><span className="dex-stat-value">{routePath}</span></div>
          )}
          {poolTvl !== undefined && (
            <div className="dex-stat"><span className="dex-stat-label">{t.dex.tvlLabel}</span><span className="dex-stat-value">${formatNumber(poolTvl)}</span></div>
          )}
        </div>
      )}

      <div className="dex-card-footer">
        <span className="dex-footer-provider">{t.dex.dataSource(providerName)}</span>
        {subscription.selected_provider_id !== 'jupiter' && (
          <span className="dex-footer-pool" title={poolAddress}>{t.dex.pool(truncateAddr(poolAddress))}</span>
        )}
      </div>

      {editing && editPanel}
    </div>
  );
});
