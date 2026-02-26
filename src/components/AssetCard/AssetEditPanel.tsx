/**
 * AssetCard 的編輯面板 — 從 AssetCard.tsx 抽出，減少主元件複雜度。
 */
import { useState, useRef, useMemo } from 'react';
import { createPortal } from 'react-dom';
import { invoke } from '@tauri-apps/api/core';
import type { Subscription, ProviderInfo } from '../../types';
import { useEscapeKey } from '../../hooks/useEscapeKey';
import { t } from '../../lib/i18n';

interface AssetEditPanelProps {
  subscription: Subscription;
  providers: ProviderInfo[];
  currentProviderId: string;
  assetType: 'crypto' | 'stock';
  isCustomView: boolean;
  onSave: (id: number, updates: { symbol?: string; displayName?: string; providerId?: string; assetType?: 'crypto' | 'stock' }) => Promise<void>;
  onRemove: (id: number) => void;
  onClose: () => void;
}

export function AssetEditPanel({ subscription, providers, currentProviderId, assetType, isCustomView, onSave, onRemove, onClose }: AssetEditPanelProps) {
  const [editSymbol, setEditSymbol] = useState(subscription.symbol);
  const [editDisplayName, setEditDisplayName] = useState(subscription.display_name || '');
  const [editProvider, setEditProvider] = useState(currentProviderId);
  const [editAssetType, setEditAssetType] = useState<'crypto' | 'stock'>(assetType);
  const [editError, setEditError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const editRef = useRef<HTMLDivElement>(null);

  useEscapeKey(onClose);

  const filteredProviders = useMemo(() => providers.filter(p =>
    editAssetType === 'crypto'
      ? (p.provider_type === 'crypto' || p.provider_type === 'both' || p.provider_type === 'dex')
      : (p.provider_type === 'stock' || p.provider_type === 'both')
  ), [providers, editAssetType]);

  const editProviderInfo = providers.find(p => p.id === editProvider);
  const isEditDex = editProviderInfo?.provider_type === 'dex';

  const handleSave = async () => {
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
      await onSave(subscription.id, { symbol: sym, displayName: editDisplayName, providerId: editProvider, assetType: editAssetType });
      onClose();
    } catch (err) {
      setEditError(t.dex.saveFailed(err instanceof Error ? err.message : String(err)));
    } finally {
      setSaving(false);
    }
  };

  return createPortal(
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-container asset-edit-panel" ref={editRef} role="dialog" aria-modal="true" onClick={e => e.stopPropagation()}>
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
          {editProviderInfo?.requires_api_key && <span className="edit-hint warning">{t.subForm.apiKeyRequired}</span>}
        </div>
        {editError && <div className="edit-error">{editError}</div>}
        <div className="edit-actions">
          <button className="edit-btn delete" onClick={() => { onRemove(subscription.id); onClose(); }}>{isCustomView ? t.subs.removeDisplay : t.common.delete}</button>
          <div className="edit-actions-right">
            <button className="edit-btn cancel" onClick={onClose} disabled={saving}>{t.common.cancel}</button>
            <button className="edit-btn save" onClick={handleSave} disabled={saving}>{saving ? t.common.saving : t.common.save}</button>
          </div>
        </div>
      </div>
    </div>,
    document.body
  );
}
