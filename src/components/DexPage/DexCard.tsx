import { useState, useEffect, memo, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Subscription, ProviderInfo } from '../../types';
import { useAssetPrice } from '../../hooks/useAssetData';
import { CountdownCircle } from '../AssetCard/CountdownCircle';
import { AssetIcon, getIconName, invalidateIcon } from '../AssetCard/AssetIcon';
import { DexEditPanel } from './DexEditPanel';
import { formatPrice, formatNumber, summarizeError, truncateAddr, parsePairFromName } from '../../lib/format';
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

  const openEdit = useCallback(() => setEditing(true), []);

  const editPanel = editing && (
    <DexEditPanel
      subscription={subscription}
      providers={providers}
      isCustomView={isCustomView}
      onSave={onEdit}
      onRemove={onRemove}
      onClose={() => setEditing(false)}
    />
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
        {editPanel}
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
        {editPanel}
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

      {editPanel}
    </div>
  );
});
