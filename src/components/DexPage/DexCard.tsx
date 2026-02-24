import { useState, useRef, useEffect, memo, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Subscription, ProviderInfo } from '../../types';
import { useAssetPrice } from '../../hooks/useAssetData';
import { CountdownCircle } from '../AssetCard/CountdownCircle';
import { AssetIcon, getIconName, invalidateIcon } from '../AssetCard/AssetIcon';
import { formatPrice, formatNumber } from '../../lib/format';
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

const DEX_PROVIDERS = [
  { id: 'jupiter', name: 'Jupiter (Solana 聚合器)' },
  { id: 'raydium', name: 'Raydium (Solana AMM)' },
  { id: 'subgraph', name: 'Subgraph (Uniswap/Sushi/Pancake)' },
];

/** 從 display_name 提取 token pair（例如 "SOL/USDC" → ["SOL", "USDC"]） */
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

  // Edit form
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

  // 從 display_name 提取 token symbol 用於 icon
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

  const providerName = DEX_PROVIDERS.find(p => p.id === subscription.selected_provider_id)?.name
    || providers.find(p => p.id === subscription.selected_provider_id)?.name
    || subscription.selected_provider_id;

  useEffect(() => {
    if (error) {
      console.warn(`[DEX ${poolAddress}@${subscription.selected_provider_id}]`, error);
      setErrorExpanded(false);
    }
  }, [error, poolAddress, subscription.selected_provider_id]);

  const openEdit = useCallback(() => {
    const isJup = subscription.selected_provider_id === 'jupiter';
    // Jupiter: 用 display_name 的 token pair 作為編輯欄位（例如 "SOL/USDC" → "SOL,USDC"）
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
    if (!pool) { setEditError(isEditJupiter ? '請輸入交易對，例如 SOL,USDC' : '請輸入 Pool 地址'); return; }
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
      // Jupiter: 自動更新暱稱
      if (editProvider === 'jupiter' && !editDisplayName) {
        setEditDisplayName(`${info.token0_symbol}/${info.token1_symbol}`);
      }
    } catch (err) {
      setEditError(`查詢失敗: ${err instanceof Error ? err.message : String(err)}`);
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
      setEditError('Pool 地址不能為空'); return;
    }
    if (!editTokenFrom.trim() || !editTokenTo.trim()) {
      setEditError('Token From、Token To 不能為空'); return;
    }
    setSaving(true);
    setEditError(null);
    const testSymbol = `${finalPool}:${editTokenFrom.trim()}:${editTokenTo.trim()}`;
    try {
      await invoke('fetch_asset_price', { providerId: editProvider, symbol: testSymbol });
    } catch (err) {
      setEditError(`驗證失敗: ${err instanceof Error ? err.message : String(err)}`);
      setSaving(false); return;
    }
    try {
      await onEdit(subscription.id, {
        poolAddress: finalPool, tokenFrom: editTokenFrom, tokenTo: editTokenTo,
        providerId: editProvider, displayName: editDisplayName,
      });
      setEditing(false);
    } catch (err) {
      setEditError(`儲存失敗: ${err instanceof Error ? err.message : String(err)}`);
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

  const editPanel = (
    <div className="dex-edit-panel" ref={editRef}>
      <div className="edit-row">
        <label>數據源</label>
        <select value={editProvider} onChange={e => setEditProvider(e.target.value)} disabled={editBusy}>
          {DEX_PROVIDERS.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}
        </select>
      </div>
      <div className="edit-row">
        <label>{isEditJupiter ? '交易對' : 'Pool 地址'}</label>
        <div style={{ display: 'flex', gap: '6px', minWidth: 0 }}>
          <input value={editPool} onChange={e => { setEditPool(e.target.value); setEditError(null); }} disabled={editBusy}
            placeholder={isEditJupiter ? 'SOL,USDC' : editProvider === 'subgraph' ? 'protocol:0x...' : 'pool address'}
            className="dex-address-input" style={{ flex: 1, minWidth: 0 }} />
          <button className="edit-btn save" onClick={handleEditLookup} disabled={editBusy} style={{ whiteSpace: 'nowrap', flexShrink: 0 }}>
            {lookingUp ? '...' : '查詢'}
          </button>
        </div>
        {isEditJupiter && <span className="edit-hint">Jupiter 自動路由，輸入代號或 mint address，逗號分隔</span>}
        {editProvider === 'subgraph' && <span className="edit-hint">Subgraph 格式: uniswap_v3:0x... 或 sushiswap:0x...</span>}
      </div>
      <div className="edit-row">
        <label>交易方向</label>
        {editManualTokens ? (
          <>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
              <input value={editTokenFrom} onChange={e => { setEditTokenFrom(e.target.value); setEditError(null); }} disabled={editBusy}
                placeholder="Token From address" className="dex-address-input" style={{ minWidth: 0 }} />
              <div style={{ display: 'flex', gap: '4px', alignItems: 'center', minWidth: 0 }}>
                <input value={editTokenTo} onChange={e => { setEditTokenTo(e.target.value); setEditError(null); }} disabled={editBusy}
                  placeholder="Token To address" className="dex-address-input" style={{ flex: 1, minWidth: 0 }} />
                <button className="edit-btn cancel" onClick={handleEditSwap} disabled={editBusy} title="翻轉方向" style={{ padding: '2px 8px', flexShrink: 0 }}>⇄</button>
              </div>
            </div>
            <button type="button" onClick={() => setEditManualTokens(false)}
              style={{ background: 'none', border: 'none', color: 'var(--blue, #89b4fa)', cursor: 'pointer', fontSize: '0.8em', padding: '2px 0 0', textAlign: 'right' }}>
              使用查詢模式
            </button>
          </>
        ) : (
          <>
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
              <span style={{ flex: 1, fontSize: '0.85em', color: 'var(--subtext0, #a6adc8)' }}>
                {editFromSymbol || truncateAddr(editTokenFrom)} → {editToSymbol || truncateAddr(editTokenTo)}
              </span>
              <button className="edit-btn cancel" onClick={handleEditSwap} disabled={editBusy} title="翻轉方向" style={{ padding: '2px 8px' }}>⇄</button>
            </div>
            <button type="button" onClick={() => setEditManualTokens(true)}
              style={{ background: 'none', border: 'none', color: 'var(--blue, #89b4fa)', cursor: 'pointer', fontSize: '0.8em', padding: '2px 0 0', textAlign: 'right' }}>
              手動修改 Token 地址
            </button>
          </>
        )}
      </div>
      <div className="edit-row">
        <label>暱稱</label>
        <input value={editDisplayName} onChange={e => setEditDisplayName(e.target.value)} placeholder="可選" disabled={editBusy} />
      </div>
      {editError && <div className="edit-error">{editError}</div>}
      <div className="edit-actions">
        <button className="edit-btn delete" onClick={() => { onRemove(subscription.id); setEditing(false); }}>
          {isCustomView ? '移除顯示' : '刪除'}
        </button>
        <div className="edit-actions-right">
          <button className="edit-btn cancel" onClick={cancelEdit} disabled={editBusy}>取消</button>
          <button className="edit-btn save" onClick={saveEdit} disabled={editBusy}>{saving ? '儲存中...' : '儲存'}</button>
        </div>
      </div>
    </div>
  );

  // Compact view
  if (viewMode === 'compact') {
    return (
      <div className="dex-card-compact">
        <div className="compact-top">
          {renderPairIcons('compact-icons')}
          <span className="compact-symbol" title={`${tokenFrom} → ${tokenTo}`}>
            {subscription.display_name || `${truncateAddr(tokenFrom)}→${truncateAddr(tokenTo)}`}
          </span>
          <button className="asset-card-edit-btn" onClick={openEdit} title="編輯">✎</button>
        </div>
        <div className="compact-bottom">
          <span className="compact-price">
            {error ? <span className="asset-error" title={error}>錯誤</span> : asset ? formatPrice(asset.price) : '-'}
          </span>
          {refreshInterval > 0 && <CountdownCircle providerId={subscription.selected_provider_id} fallbackInterval={refreshInterval} size={16} />}
        </div>
        {editing && editPanel}
      </div>
    );
  }

  // List view
  if (viewMode === 'list') {
    return (
      <div className="dex-card-list">
        {renderPairIcons('list-icons')}
        <div className="dex-list-symbol">
          <span className="symbol" title={`${tokenFrom} → ${tokenTo}`}>
            {subscription.display_name || `${truncateAddr(tokenFrom)} → ${truncateAddr(tokenTo)}`}
          </span>
          <span className="dex-pool-addr" title={poolAddress}>
            {subscription.selected_provider_id !== 'jupiter' ? `Pool: ${truncateAddr(poolAddress)}` : 'Jupiter 聚合'}
          </span>
        </div>
        <div className="dex-list-price">
          {error ? <span className="asset-error" title={error}>錯誤</span> : asset ? formatPrice(asset.price) : '載入中...'}
        </div>
        {amountOut !== undefined && (
          <div className="dex-list-swap">1 → {amountOut.toPrecision(6)}</div>
        )}
        <span className="dex-list-provider">數據源: {providerName}</span>
        <button className="asset-card-edit-btn" onClick={openEdit} title="編輯">✎</button>
        {refreshInterval > 0 && <CountdownCircle providerId={subscription.selected_provider_id} fallbackInterval={refreshInterval} size={22} />}
        {editing && editPanel}
      </div>
    );
  }

  // Grid view (default)
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
        <button className="asset-card-edit-btn" onClick={openEdit} title="編輯">✎</button>
        {refreshInterval > 0 && <CountdownCircle providerId={subscription.selected_provider_id} fallbackInterval={refreshInterval} size={20} />}
      </div>

      <div className="dex-card-body">
        <p className="dex-price">
          {error ? <span className="asset-error">獲取失敗</span> : asset ? formatPrice(asset.price) : '載入中...'}
        </p>
        {amountOut !== undefined && !error && (
          <p className="dex-swap-rate">1 token → {amountOut.toPrecision(6)}</p>
        )}
      </div>

      {error && (
        <div className="dex-error-detail" onClick={() => setErrorExpanded(v => !v)} title="點擊展開/收起">
          <span className="dex-error-summary">{error.length > 60 ? error.slice(0, 57) + '...' : error}</span>
          {errorExpanded && <pre className="dex-error-full">{error}</pre>}
        </div>
      )}

      {asset && !error && (
        <div className="dex-card-stats">
          {gasEstimate && (
            <div className="dex-stat"><span className="dex-stat-label">Gas</span><span className="dex-stat-value">{gasEstimate}</span></div>
          )}
          {routePath && (
            <div className="dex-stat"><span className="dex-stat-label">路徑</span><span className="dex-stat-value">{routePath}</span></div>
          )}
          {poolTvl !== undefined && (
            <div className="dex-stat"><span className="dex-stat-label">TVL</span><span className="dex-stat-value">${formatNumber(poolTvl)}</span></div>
          )}
        </div>
      )}

      <div className="dex-card-footer">
        <span className="dex-footer-provider">數據源: {providerName}</span>
        {subscription.selected_provider_id !== 'jupiter' && (
          <span className="dex-footer-pool" title={poolAddress}>Pool: {truncateAddr(poolAddress)}</span>
        )}
      </div>

      {editing && editPanel}
    </div>
  );
});
