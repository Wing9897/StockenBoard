import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

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
    if (!pool) { setError(isJupiter ? '請輸入交易對，例如 SOL,USDC' : '請輸入 Pool 地址'); return; }
    setLooking(true);
    setError(null);
    setPoolInfo(null);
    setSwapped(false);
    setManualMode(false);
    try {
      let lookupAddr = pool;
      if (provider === 'subgraph') lookupAddr = `${protocol}:${pool}`;
      // Jupiter: pool 欄位就是 "SOL,USDC" 格式，直接傳給 lookup_dex_pool
      const info = await invoke<DexPoolInfo>('lookup_dex_pool', { providerId: provider, poolAddress: lookupAddr });
      setPoolInfo(info);
      if (!displayName) setDisplayName(`${info.token0_symbol}/${info.token1_symbol}`);
    } catch (err) {
      setError(`查詢失敗: ${err instanceof Error ? err.message : String(err)}。可改用手動輸入。`);
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
        // 翻轉後: newSwapped=true → token1/token0, newSwapped=false → token0/token1
        const s0 = newSwapped ? poolInfo.token1_symbol : poolInfo.token0_symbol;
        const s1 = newSwapped ? poolInfo.token0_symbol : poolInfo.token1_symbol;
        setDisplayName(`${s0}/${s1}`);
      }
    }
  };

  const handleSubmit = async () => {
    if (!hasTokens) { setError('請先查詢或手動輸入 Token 地址'); return; }
    setSubmitting(true);
    setError(null);

    // Jupiter: pool = "auto", Subgraph: pool = "protocol:0x...", Raydium: pool = address
    let finalPool: string;
    if (isJupiter) {
      finalPool = 'auto';
    } else if (provider === 'subgraph') {
      finalPool = `${protocol}:${poolAddress.trim()}`;
    } else {
      finalPool = poolAddress.trim();
    }
    if (!isJupiter && !finalPool) { setError('Pool 地址不能為空'); setSubmitting(false); return; }

    const testSymbol = `${finalPool}:${tokenFrom}:${tokenTo}`;
    try {
      await invoke('fetch_asset_price', { providerId: provider, symbol: testSymbol });
    } catch (err) {
      setError(`驗證失敗: ${err instanceof Error ? err.message : String(err)}`);
      setSubmitting(false);
      return;
    }

    try {
      const label = fromSymbol && toSymbol ? `${fromSymbol}/${toSymbol}` : `${tokenFrom.slice(0, 8)}.../${tokenTo.slice(0, 8)}...`;
      await onAdd(finalPool, tokenFrom, tokenTo, provider, displayName || label);
      onToast?.('success', '已新增 DEX 訂閱', label);
      onClose();
    } catch (err) {
      setError(`新增失敗: ${err instanceof Error ? err.message : String(err)}`);
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

  const poolLabel = isJupiter ? '交易對' : 'Pool 地址';
  const poolPlaceholder = isJupiter
    ? 'SOL,USDC 或 mintAddress,mintAddress'
    : provider === 'raydium'
      ? 'Raydium pool address'
      : '0x... pool address';

  return (
    <div className="sub-modal-backdrop" onClick={onClose}>
      <div className="sub-modal" onClick={e => e.stopPropagation()}>
        <div className="sub-modal-header">
          <h4 className="sub-modal-title">新增 DEX 訂閱</h4>
          <button className="vsm-close" onClick={onClose}>✕</button>
        </div>
        <div className="sub-modal-body">
          <div className="dex-form">
            <div className="dex-form-row">
              <label>數據源</label>
              <select value={provider} onChange={e => handleProviderChange(e.target.value)} disabled={busy}>
                <option value="jupiter">Jupiter (Solana 聚合器)</option>
                <option value="raydium">Raydium (Solana AMM)</option>
                <option value="subgraph">Subgraph (EVM DEX)</option>
              </select>
            </div>

            {provider === 'subgraph' && (
              <div className="dex-form-row">
                <label>DEX 協議</label>
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
                    {looking ? '查詢中...' : '查詢'}
                  </button>
                )}
              </div>
              {isJupiter && <span className="edit-hint">Jupiter 自動路由，輸入代號或 mint address，逗號分隔</span>}
              {provider === 'subgraph' && <span className="edit-hint">Subgraph 格式: 0x... pool address</span>}
            </div>

            {/* Auto-lookup result */}
            {!manualMode && poolInfo && (
              <div className="dex-form-row">
                <label>交易方向</label>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <span className="dex-token-badge">{fromSymbol} ({tokenFrom.slice(0, 8)}...)</span>
                  <button type="button" onClick={handleSwap} disabled={busy}
                    style={{ background: 'none', border: '1px solid var(--surface1, #45475a)', borderRadius: '4px', cursor: 'pointer', padding: '4px 8px', color: 'var(--text, #cdd6f4)' }}
                    title="翻轉方向">⇄</button>
                  <span className="dex-token-badge">{toSymbol} ({tokenTo.slice(0, 8)}...)</span>
                </div>
              </div>
            )}

            {/* Manual input fields */}
            {manualMode && (
              <>
                <div className="dex-form-row">
                  <label>Token From 地址</label>
                  <input value={manualTokenFrom} onChange={e => { setManualTokenFrom(e.target.value); setError(null); }}
                    placeholder={provider === 'subgraph' ? '0x... token address' : 'Solana mint address'}
                    className="dex-address-input" disabled={busy} />
                </div>
                <div className="dex-form-row">
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <label>Token To 地址</label>
                    <button type="button" onClick={handleSwap} disabled={busy}
                      style={{ background: 'none', border: '1px solid var(--surface1, #45475a)', borderRadius: '4px', cursor: 'pointer', padding: '2px 6px', fontSize: '0.8em', color: 'var(--text, #cdd6f4)' }}
                      title="翻轉方向">⇄ 翻轉</button>
                  </div>
                  <input value={manualTokenTo} onChange={e => { setManualTokenTo(e.target.value); setError(null); }}
                    placeholder={provider === 'subgraph' ? '0x... token address' : 'Solana mint address'}
                    className="dex-address-input" disabled={busy} />
                </div>
              </>
            )}

            {/* Mode switch link */}
            {!manualMode && !poolInfo && !looking && (
              <div className="dex-form-row" style={{ textAlign: 'right' }}>
                <button type="button" onClick={switchToManual}
                  style={{ background: 'none', border: 'none', color: 'var(--blue, #89b4fa)', cursor: 'pointer', fontSize: '0.85em', padding: 0 }}>
                  手動輸入 Token 地址
                </button>
              </div>
            )}
            {manualMode && (
              <div className="dex-form-row" style={{ textAlign: 'right' }}>
                <button type="button" onClick={switchToAuto}
                  style={{ background: 'none', border: 'none', color: 'var(--blue, #89b4fa)', cursor: 'pointer', fontSize: '0.85em', padding: 0 }}>
                  改用自動查詢
                </button>
              </div>
            )}

            {hasTokens && (
              <div className="dex-form-row">
                <label>顯示暱稱（可選）</label>
                <input value={displayName} onChange={e => setDisplayName(e.target.value)}
                  placeholder="例如: SOL/USDC" disabled={busy} />
              </div>
            )}

            {error && <div className="dex-form-error">{error}</div>}

            {hasTokens && (
              <button className="dex-form-submit" onClick={handleSubmit} disabled={busy}>
                {submitting ? '驗證中...' : '新增訂閱'}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
