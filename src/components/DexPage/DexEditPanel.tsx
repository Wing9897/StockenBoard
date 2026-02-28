/**
 * DexCard 的編輯面板 — 使用 EditPanelShell 共用外殼
 */
import { useState, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Subscription, ProviderInfo } from '../../types';
import { truncateAddr } from '../../lib/format';
import { EditPanelShell } from '../EditPanel/EditPanelShell';
import { t } from '../../lib/i18n';

interface DexEditPanelProps {
  subscription: Subscription;
  providers: ProviderInfo[];
  isCustomView: boolean;
  onSave: (id: number, updates: {
    poolAddress?: string; tokenFrom?: string; tokenTo?: string;
    providerId?: string; displayName?: string;
  }) => Promise<void>;
  onRemove: (id: number) => void;
  onClose: () => void;
}

export function DexEditPanel({ subscription, providers, isCustomView, onSave, onRemove, onClose }: DexEditPanelProps) {
  const poolAddress = subscription.pool_address || '';
  const tokenFrom = subscription.token_from_address || '';
  const tokenTo = subscription.token_to_address || '';

  const isJupInit = subscription.selected_provider_id === 'jupiter';
  const [editPool, setEditPool] = useState(isJupInit ? '' : poolAddress);
  const [editTokenFrom, setEditTokenFrom] = useState(tokenFrom);
  const [editTokenTo, setEditTokenTo] = useState(tokenTo);
  const [editFromSymbol, setEditFromSymbol] = useState('');
  const [editToSymbol, setEditToSymbol] = useState('');
  const [editProvider, setEditProvider] = useState(subscription.selected_provider_id);
  const [editDisplayName, setEditDisplayName] = useState(subscription.display_name || '');
  const [editError, setEditError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [lookingUp, setLookingUp] = useState(false);
  const [manualTokens, setManualTokens] = useState(false);

  const editBusy = saving || lookingUp;
  const isEditJupiter = editProvider === 'jupiter';
  const dexProviders = useMemo(() => providers.filter(p => p.provider_type === 'dex'), [providers]);

  const handleLookup = async () => {
    const pool = editPool.trim();
    if (!pool) { setEditError(isEditJupiter ? t.errors.pairInputRequired : t.errors.poolInputRequired); return; }
    setLookingUp(true); setEditError(null);
    try {
      const info = await invoke<{ token0_address: string; token0_symbol: string; token1_address: string; token1_symbol: string }>(
        'lookup_dex_pool', { providerId: editProvider, poolAddress: pool }
      );
      setEditTokenFrom(info.token0_address); setEditTokenTo(info.token1_address);
      setEditFromSymbol(info.token0_symbol); setEditToSymbol(info.token1_symbol);
      if (isEditJupiter && !editDisplayName) setEditDisplayName(`${info.token0_symbol}/${info.token1_symbol}`);
    } catch (err) {
      setEditError(t.dex.lookupFailed(err instanceof Error ? err.message : String(err)) + t.dex.lookupFailedManualHint);
    } finally { setLookingUp(false); }
  };

  const handleSwap = () => {
    setEditTokenFrom(editTokenTo); setEditTokenTo(editTokenFrom);
    setEditFromSymbol(editToSymbol); setEditToSymbol(editFromSymbol);
  };

  const handleSave = async () => {
    const isJup = editProvider === 'jupiter';
    const finalPool = isJup ? 'auto' : editPool.trim();
    if (!isJup && !finalPool) { setEditError(t.dex.poolEmpty); return; }
    if (!editTokenFrom.trim() || !editTokenTo.trim()) { setEditError(t.dex.tokenEmpty); return; }
    setSaving(true); setEditError(null);
    const testSymbol = `${finalPool}:${editTokenFrom.trim()}:${editTokenTo.trim()}`;
    try {
      await invoke('fetch_asset_price', { providerId: editProvider, symbol: testSymbol });
    } catch (err) {
      setEditError(t.dex.validateFailed(err instanceof Error ? err.message : String(err)));
      setSaving(false); return;
    }
    try {
      await onSave(subscription.id, {
        poolAddress: finalPool, tokenFrom: editTokenFrom, tokenTo: editTokenTo,
        providerId: editProvider, displayName: editDisplayName,
      });
      onClose();
    } catch (err) {
      setEditError(t.dex.saveFailed(err instanceof Error ? err.message : String(err)));
    } finally { setSaving(false); }
  };

  return (
    <EditPanelShell
      className="dex-edit-panel"
      error={editError}
      saving={editBusy}
      isCustomView={isCustomView}
      onSave={handleSave}
      onDelete={() => { onRemove(subscription.id); onClose(); }}
      onClose={onClose}
    >
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
          <button className="edit-btn save" onClick={handleLookup} disabled={editBusy}>
            {lookingUp ? t.dex.lookingUp : t.dex.lookup}
          </button>
        </div>
        {isEditJupiter && <span className="edit-hint">{t.dex.jupiterHint}</span>}
        {editProvider === 'subgraph' && <span className="edit-hint">{t.dex.subgraphHint}</span>}
      </div>
      <div className="edit-row">
        <label>{t.dex.tradeDirection}</label>
        {manualTokens ? (
          <>
            <div className="dex-edit-token-col">
              <input value={editTokenFrom} onChange={e => { setEditTokenFrom(e.target.value); setEditError(null); }} disabled={editBusy}
                placeholder={t.dex.tokenFromPlaceholder} className="dex-address-input" />
              <div className="dex-edit-token-input-row">
                <input value={editTokenTo} onChange={e => { setEditTokenTo(e.target.value); setEditError(null); }} disabled={editBusy}
                  placeholder={t.dex.tokenToPlaceholder} className="dex-address-input" />
                <button className="edit-btn cancel dex-edit-swap-sm" onClick={handleSwap} disabled={editBusy} title={t.dex.flipDirection}>⇄</button>
              </div>
            </div>
            <button type="button" className="dex-edit-link-btn" onClick={() => setManualTokens(false)}>{t.dex.useAutoMode}</button>
          </>
        ) : (
          <>
            <div className="dex-edit-direction-row">
              <span className="dex-edit-direction-text">
                {editFromSymbol || truncateAddr(editTokenFrom)} → {editToSymbol || truncateAddr(editTokenTo)}
              </span>
              <button className="edit-btn cancel dex-edit-swap-sm" onClick={handleSwap} disabled={editBusy} title={t.dex.flipDirection}>⇄</button>
            </div>
            <button type="button" className="dex-edit-link-btn" onClick={() => setManualTokens(true)}>{t.dex.useManualMode}</button>
          </>
        )}
      </div>
      <div className="edit-row">
        <label>{t.dex.nickname}</label>
        <input value={editDisplayName} onChange={e => setEditDisplayName(e.target.value)} placeholder={t.dex.nicknameOptional} disabled={editBusy} />
      </div>
    </EditPanelShell>
  );
}
