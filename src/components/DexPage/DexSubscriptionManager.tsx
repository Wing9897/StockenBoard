import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { t } from '../../lib/i18n';

const PROTOCOLS = [
  { id: 'uniswap_v3', name: 'Uniswap V3' },
  { id: 'sushiswap', name: 'SushiSwap' },
  { id: 'pancakeswap', name: 'PancakeSwap' },
];

interface DexPoolInfo {
  token0_address: string;
  token0_symbol: string;
  token1_address: string;
  token1_symbol: string;
}

interface DexSubscriptionManagerProps {
  onAdd: (poolAddress: string, tokenFrom: string, tokenTo: string, providerId: string, displayName?: string) => Promise<void>;
  onToast?: (type: 'success' | 'error' | 'info', title: string, msg?: string) => void;
  onClose: () => void;
}

export function DexSubscriptionManager({ onAdd, onToast, onClose }: DexSubscriptionManagerProps) {
  const [provider, setProvider] = useState('jupiter');
  const [protocol, setProtocol] = useState('uniswap_v3');
  const [poolAddress, setPoolAddress] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [looking, setLooking] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  const [poolInfo, setPoolInfo] = useState<DexPoolInfo | null>(null);
  const [swapped, setSwapped] = useState(false);

  const [manualMode, setManualMode] = useState(false);
  const [manualTokenFrom, setManualTokenFrom] = useState('');
  const [manualTokenTo, setManualTokenTo] = useState('');

  const isJupiter = provider === 'jupiter';

  const tokenFrom = manualMode
    ? manualTokenFrom.trim()
    : poolInfo ? (swapped ? poolInfo.token1_address : poolInfo.token0_address) : '';
  const tokenTo = manualMode
    ? manualTokenTo.trim()
    : poolInfo ? (swapped ? poolInfo.token0_address : poolInfo.token1_address) : '';
  const fromSymbol = poolInfo ? (swapped ? poolInfo.token1_symbol : poolInfo.token0_symbol) : '';
  const toSymbol = poolInfo ? (swapped ? poolInfo.token0_symbol : poolInfo.token1_symbol) : '';

  const hasTokens = manualMode ? (!!tokenFrom && !!tokenTo) : !!poolInfo;

  const handleLookup = async () => {
    const pool = poolAddress.trim();
    if (!pool) { setError(isJupiter ? t.errors.pairInputRequired : t.errors.poolInputRequired); return; }
    setLooking(true);
    setError(null);
    setPoolInfo(null);
    setSwapped(false);
    setManualMode(false);
    try {
      let lookupAddr = pool;
      if (provider === 'subgraph') lookupAddr = `${protocol}:${pool}`;
      const info = await invoke<DexPoolInfo>('lookup_dex_pool', { providerId: provider, poolAddress: lookupAddr });
      setPoolInfo(info);
      if (!displayName) setDisplayName(`${info.token0_symbol}/${info.token1_symbol}`);
    } catch (err) {
      setError(`${t.dex.lookupFailed(err instanceof Error ? err.message : String(err))}${t.dex.lookupFailedManualHint}`);
    } finally {
      setLooking(false);
    }
  };

  const handleSwap = () => {
    if (manualMode) {
      const tmp = manualTokenFrom;
      setManualTokenFrom(manualTokenTo);
      setManualTokenTo(tmp);
    } else {
      const newSwapped = !swapped;
      setSwapped(newSwapped);
      if (poolInfo && displayName) {
        const s0 = newSwapped ? poolInfo.token1_symbol : poolInfo.token0_symbol;
        const s1 = newSwapped ? poolInfo.token0_symbol : poolInfo.token1_symbol;
        setDisplayName(`${s0}/${s1}`);
      }
    }
  };

  const handleSubmit = async () => {
    if (!hasTokens) { setError(t.dex.noTokens); return; }
    setSubmitting(true);
    setError(null);

    let finalPool: string;
    if (isJupiter) {
      finalPool = 'auto';
    } else if (provider === 'subgraph') {
      finalPool = `${protocol}:${poolAddress.trim()}`;
    } else {
      finalPool = poolAddress.trim();
    }
    if (!isJupiter && !finalPool) { setError(t.dex.poolEmpty); setSubmitting(false); return; }

    const testSymbol = `${finalPool}:${tokenFrom}:${tokenTo}`;
    try {
      await invoke('fetch_asset_price', { providerId: provider, symbol: testSymbol });
    } catch (err) {
      setError(t.dex.validateFailed(err instanceof Error ? err.message : String(err)));
      setSubmitting(false);
      return;
    }

    try {
      const label = fromSymbol && toSymbol ? `${fromSymbol}/${toSymbol}` : `${tokenFrom.slice(0, 8)}.../${tokenTo.slice(0, 8)}...`;
      await onAdd(finalPool, tokenFrom, tokenTo, provider, displayName || label);
      onToast?.('success', t.dex.addedDex, label);
      onClose();
    } catch (err) {
      setError(t.dex.saveFailed(err instanceof Error ? err.message : String(err)));
    } finally {
      setSubmitting(false);
    }
  };

  const handleProviderChange = (newProvider: string) => {
    setProvider(newProvider);
    setPoolInfo(null);
    setSwapped(false);
    setManualMode(false);
    setManualTokenFrom('');
    setManualTokenTo('');
    setError(null);
    setDisplayName('');
    setPoolAddress('');
  };

  const switchToManual = () => {
    setManualMode(true);
    setPoolInfo(null);
    setSwapped(false);
    setError(null);
  };

  const switchToAuto = () => {
    setManualMode(false);
    setManualTokenFrom('');
    setManualTokenTo('');
    setError(null);
  };

  const busy = looking || submitting;

  const poolLabel = isJupiter ? t.dex.tradePair : t.dex.poolAddress;
  const poolPlaceholder = isJupiter
    ? t.dex.jupiterPoolPlaceholder
    : provider === 'raydium'
      ? t.dex.raydiumPoolPlaceholder
      : t.dex.evmPoolPlaceholder;

  return (
    <div className="modal-backdrop sub-modal-backdrop" onClick={onClose}>
      <div className="modal-container sub-modal" onClick={e => e.stopPropagation()}>
        <div className="sub-modal-header">
          <h4 className="sub-modal-title">{t.dex.addDexSub}</h4>
          <button className="vsm-close" onClick={onClose}>✕</button>
        </div>
        <div className="sub-modal-body">
          <div className="dex-form">
            <div className="dex-form-row">
              <label>{t.dex.provider}</label>
              <select value={provider} onChange={e => handleProviderChange(e.target.value)} disabled={busy}>
                <option value="jupiter">{t.dex.jupiterProviderLabel}</option>
                <option value="raydium">{t.dex.raydiumProviderLabel}</option>
                <option value="subgraph">{t.dex.subgraphProviderLabel}</option>
              </select>
            </div>

            {provider === 'subgraph' && (
              <div className="dex-form-row">
                <label>{t.dex.protocol}</label>
                <select value={protocol} onChange={e => { setProtocol(e.target.value); setPoolInfo(null); setManualMode(false); }} disabled={busy}>
                  {PROTOCOLS.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}
                </select>
              </div>
            )}

            <div className="dex-form-row">
              <label>{poolLabel}</label>
              <div style={{ display: 'flex', gap: '8px', minWidth: 0 }}>
                <input
                  value={poolAddress}
                  onChange={e => { setPoolAddress(e.target.value); setError(null); if (!manualMode) setPoolInfo(null); }}
                  placeholder={poolPlaceholder}
                  className="dex-address-input"
                  style={{ flex: 1, minWidth: 0 }}
                  disabled={busy}
                />
                {!manualMode && (
                  <button className="dex-form-submit" onClick={handleLookup} disabled={busy}
                    style={{ whiteSpace: 'nowrap', minWidth: 'auto', padding: '6px 12px', flexShrink: 0 }}>
                    {looking ? t.dex.lookingUp : t.dex.lookup}
                  </button>
                )}
              </div>
              {isJupiter && <span className="edit-hint">{t.dex.jupiterHint}</span>}
              {provider === 'subgraph' && <span className="edit-hint">{t.dex.subgraphPoolHint}</span>}
            </div>

            {!manualMode && poolInfo && (
              <div className="dex-form-row">
                <label>{t.dex.tradeDirection}</label>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <span className="dex-token-badge">{fromSymbol} ({tokenFrom.slice(0, 8)}...)</span>
                  <button type="button" onClick={handleSwap} disabled={busy}
                    style={{ background: 'none', border: '1px solid var(--surface1)', borderRadius: '4px', cursor: 'pointer', padding: '4px 8px', color: 'var(--text)' }}
                    title={t.dex.flipDirection}>⇄</button>
                  <span className="dex-token-badge">{toSymbol} ({tokenTo.slice(0, 8)}...)</span>
                </div>
              </div>
            )}

            {manualMode && (
              <>
                <div className="dex-form-row">
                  <label>{t.dex.tokenFromLabel}</label>
                  <input value={manualTokenFrom} onChange={e => { setManualTokenFrom(e.target.value); setError(null); }}
                    placeholder={provider === 'subgraph' ? t.dex.evmTokenPlaceholder : t.dex.solanaTokenPlaceholder}
                    className="dex-address-input" disabled={busy} />
                </div>
                <div className="dex-form-row">
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <label>{t.dex.tokenToLabel}</label>
                    <button type="button" onClick={handleSwap} disabled={busy}
                      style={{ background: 'none', border: '1px solid var(--surface1)', borderRadius: '4px', cursor: 'pointer', padding: '2px 6px', fontSize: '0.8em', color: 'var(--text)' }}
                      title={t.dex.flipDirection}>⇄ {t.dex.flipShort}</button>
                  </div>
                  <input value={manualTokenTo} onChange={e => { setManualTokenTo(e.target.value); setError(null); }}
                    placeholder={provider === 'subgraph' ? t.dex.evmTokenPlaceholder : t.dex.solanaTokenPlaceholder}
                    className="dex-address-input" disabled={busy} />
                </div>
              </>
            )}

            {!manualMode && !poolInfo && !looking && (
              <div className="dex-form-row" style={{ textAlign: 'right' }}>
                <button type="button" onClick={switchToManual}
                  style={{ background: 'none', border: 'none', color: 'var(--blue)', cursor: 'pointer', fontSize: '0.85em', padding: 0 }}>
                  {t.dex.manualInput}
                </button>
              </div>
            )}
            {manualMode && (
              <div className="dex-form-row" style={{ textAlign: 'right' }}>
                <button type="button" onClick={switchToAuto}
                  style={{ background: 'none', border: 'none', color: 'var(--blue)', cursor: 'pointer', fontSize: '0.85em', padding: 0 }}>
                  {t.dex.switchToAuto}
                </button>
              </div>
            )}

            {hasTokens && (
              <div className="dex-form-row">
                <label>{t.dex.nickname}({t.dex.nicknameOptional})</label>
                <input value={displayName} onChange={e => setDisplayName(e.target.value)}
                  placeholder={t.dex.displayNamePlaceholder} disabled={busy} />
              </div>
            )}

            {error && <div className="dex-form-error">{error}</div>}

            {hasTokens && (
              <button className="dex-form-submit" onClick={handleSubmit} disabled={busy}>
                {submitting ? t.dex.verifying : t.subs.addSub}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
