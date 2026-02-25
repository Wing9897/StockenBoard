import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ProviderInfo, Subscription } from '../../types';
import { getDb } from '../../lib/db';
import { t } from '../../lib/i18n';
import './Settings.css';

interface BatchResult {
  succeeded: string[];
  failed: string[];
  duplicates: string[];
}

interface SubscriptionManagerProps {
  onBatchAdd: (symbol: string, providerId?: string, assetType?: 'crypto' | 'stock') => Promise<void>;
  subscriptions: Subscription[];
  providers: ProviderInfo[];
  onToast?: (type: 'success' | 'error' | 'info', title: string, message?: string) => void;
}

async function hasApiKey(providerId: string): Promise<boolean> {
  try {
    const db = await getDb();
    const rows = await db.select<{ api_key: string | null }[]>(
      'SELECT api_key FROM provider_settings WHERE provider_id = $1',
      [providerId]
    );
    return rows.length > 0 && !!rows[0].api_key;
  } catch { return false; }
}

async function saveApiKey(providerId: string, apiKey: string, apiSecret?: string) {
  const db = await getDb();
  await db.execute(
    `INSERT INTO provider_settings (provider_id, api_key, api_secret, connection_type, enabled)
     VALUES ($1, $2, $3, 'rest', 1)
     ON CONFLICT(provider_id) DO UPDATE SET api_key = $2, api_secret = $3, enabled = 1`,
    [providerId, apiKey || null, apiSecret || null]
  );
  await invoke('enable_provider', {
    providerId,
    apiKey: apiKey || null,
    apiSecret: apiSecret || null,
  });
}

export function SubscriptionManager({ onBatchAdd, subscriptions, providers: providerInfoList, onToast }: SubscriptionManagerProps) {
  const [symbolInput, setSymbolInput] = useState('');
  const [assetType, setAssetType] = useState<'crypto' | 'stock'>('crypto');
  const [provider, setProvider] = useState('binance');
  const [importing, setImporting] = useState(false);
  const [importStatus, setImportStatus] = useState<{ done: number; total: number } | null>(null);
  const [batchResult, setBatchResult] = useState<BatchResult | null>(null);

  const [apiKeyInput, setApiKeyInput] = useState('');
  const [apiSecretInput, setApiSecretInput] = useState('');
  const [keySaved, setKeySaved] = useState(false);
  const [keySaving, setKeySaving] = useState(false);

  const filteredProviders = providerInfoList.filter(p =>
    assetType === 'crypto'
      ? (p.provider_type === 'crypto' || p.provider_type === 'both' || p.provider_type === 'dex')
      : (p.provider_type === 'stock' || p.provider_type === 'both')
  );

  useEffect(() => {
    setProvider(assetType === 'crypto' ? 'binance' : 'yahoo');
  }, [assetType]);

  const selectedProviderInfo = providerInfoList.find(p => p.id === provider);
  const isDex = selectedProviderInfo?.provider_type === 'dex';
  const needsKey = selectedProviderInfo?.requires_api_key || false;
  const optionalKey = selectedProviderInfo?.optional_api_key || false;
  const needsSecret = selectedProviderInfo?.requires_api_secret || false;
  const showKeyInput = needsKey || optionalKey;

  useEffect(() => {
    setApiKeyInput('');
    setApiSecretInput('');
    setKeySaved(false);
    if (showKeyInput) {
      hasApiKey(provider).then(setKeySaved);
    }
  }, [provider, showKeyInput]);

  const examples = selectedProviderInfo
    ? selectedProviderInfo.symbol_format.split(/[,，]\s*/).map(s => s.trim()).filter(Boolean)
    : [];

  const handleSaveKey = async () => {
    if (!apiKeyInput.trim()) return;
    setKeySaving(true);
    try {
      await saveApiKey(provider, apiKeyInput.trim(), needsSecret ? apiSecretInput.trim() : undefined);
      setKeySaved(true);
      onToast?.('success', t.apiKey.keySaved, t.apiKey.keySavedMsg(selectedProviderInfo?.name || ''));
    } catch (err) {
      onToast?.('error', t.apiKey.saveFailed, err instanceof Error ? err.message : String(err));
    } finally {
      setKeySaving(false);
    }
  };

  const handleImport = async (e: React.FormEvent) => {
    e.preventDefault();

    if (needsKey && !keySaved && apiKeyInput.trim()) {
      await handleSaveKey();
    }

    const symbols = symbolInput
      .split(/[,\n\r;]+/)
      .map(s => s.trim())
      .filter(s => s.length > 0);
    if (symbols.length === 0) return;

    const existing = new Set(subscriptions.map(s => isDex ? s.symbol : s.symbol.toUpperCase()));
    const unique = symbols.filter(s => !existing.has(isDex ? s : s.toUpperCase()));
    const duplicates = symbols.filter(s => existing.has(isDex ? s : s.toUpperCase()));

    if (symbols.length === 1) {
      if (duplicates.length === 1) {
        onToast?.('info', t.subForm.alreadyExists, t.subForm.alreadyExistsMsg(duplicates[0]));
        return;
      }
      setImporting(true);
      setImportStatus({ done: 0, total: 1 });
      try {
        await onBatchAdd(unique[0], provider, assetType);
        onToast?.('success', t.subForm.added, t.subForm.addedMsg(isDex ? unique[0] : unique[0].toUpperCase()));
        setSymbolInput('');
      } catch {
        onToast?.('error', t.subForm.addFailed, t.subForm.addFailedMsg(unique[0]));
      }
      setImporting(false);
      setImportStatus(null);
      return;
    }

    setImporting(true);
    const succeeded: string[] = [];
    const failed: string[] = [];
    setImportStatus({ done: 0, total: unique.length });

    for (const sym of unique) {
      try {
        await onBatchAdd(sym, provider, assetType);
        succeeded.push(sym);
      } catch {
        failed.push(sym);
      }
      setImportStatus({ done: succeeded.length + failed.length, total: unique.length });
    }

    setImporting(false);
    setImportStatus(null);

    setBatchResult({ succeeded, failed, duplicates });
    if (failed.length === 0) setSymbolInput('');
  };

  return (
    <div className="settings-section">
      <h3>{t.subs.addSub}</h3>

      <form className="subscription-form" onSubmit={handleImport}>
        <div className="form-group">
          <label>{t.subForm.assetType}</label>
          <div className="asset-type-toggle">
            <button type="button" className={`type-btn ${assetType === 'crypto' ? 'active' : ''}`} onClick={() => setAssetType('crypto')}>{t.subForm.crypto}</button>
            <button type="button" className={`type-btn ${assetType === 'stock' ? 'active' : ''}`} onClick={() => setAssetType('stock')}>{t.subForm.stock}</button>
          </div>
        </div>
        <div className="form-group">
          <label>{t.subForm.defaultProvider}</label>
          <select value={provider} onChange={(e) => setProvider(e.target.value)}>
            {filteredProviders.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name} {p.requires_api_key ? t.subForm.needsApiKey : p.optional_api_key ? t.subForm.optionalKey : ''}
              </option>
            ))}
          </select>
          {selectedProviderInfo && <span className="form-hint">{(t.providerDesc as Record<string, string>)?.[provider] || selectedProviderInfo.free_tier_info}</span>}
        </div>

        {showKeyInput && (
          <div className="form-group api-key-group">
            <label>
              {t.apiKey.label}
              {optionalKey && !needsKey && <span className="optional-tag">{t.apiKey.optional}</span>}
              {needsKey && <span className="required-tag">{t.apiKey.required}</span>}
              {keySaved && <span className="saved-tag">{t.apiKey.saved}</span>}
            </label>
            {keySaved ? (
              <div className="key-saved-row">
                <span className="key-saved-text">{t.apiKey.alreadySet}</span>
                <button type="button" className="btn-change-key" onClick={() => { setKeySaved(false); setApiKeyInput(''); setApiSecretInput(''); }}>
                  {t.apiKey.change}
                </button>
              </div>
            ) : (
              <>
                <input
                  type="password"
                  value={apiKeyInput}
                  onChange={(e) => setApiKeyInput(e.target.value)}
                  placeholder={t.apiKey.placeholder}
                  disabled={importing || keySaving}
                />
                {needsSecret && (
                  <input
                    type="password"
                    value={apiSecretInput}
                    onChange={(e) => setApiSecretInput(e.target.value)}
                    placeholder={t.apiKey.secretPlaceholder}
                    disabled={importing || keySaving}
                    style={{ marginTop: '4px' }}
                  />
                )}
                {apiKeyInput.trim() && (
                  <button type="button" className="btn-save-key" onClick={handleSaveKey} disabled={keySaving}>
                    {keySaving ? t.common.saving : t.apiKey.saveKey}
                  </button>
                )}
              </>
            )}
          </div>
        )}

        <div className="form-group">
          <label>{isDex ? t.subForm.symbolDex : t.subForm.symbol}</label>
          {isDex ? (
            <textarea
              value={symbolInput}
              onChange={(e) => setSymbolInput(e.target.value)}
              placeholder={examples.length > 0 ? t.subForm.example(examples.join(', ')) : t.subForm.inputSymbolOrAddr}
              required
              disabled={importing}
              rows={3}
            />
          ) : (
            <input
              type="text"
              value={symbolInput}
              onChange={(e) => setSymbolInput(e.target.value)}
              placeholder={examples.length > 0 ? t.subForm.example(examples.join(', ')) : t.subForm.inputSymbol}
              required
              disabled={importing}
            />
          )}
          {isDex && provider === 'jupiter' && (
            <span className="form-hint">{t.subForm.jupiterHint}</span>
          )}
          {isDex && provider === 'okx_dex' && (
            <span className="form-hint">{t.subForm.okxDexHint}</span>
          )}
          {!isDex && <span className="form-hint">{t.subForm.multiSymbolHint}</span>}
        </div>
        {importStatus && (
          <div className="batch-status">
            <span>{t.subForm.importProgress(importStatus.done, importStatus.total)}</span>
          </div>
        )}
        <button type="submit" className="btn-add" disabled={!symbolInput.trim() || importing || (needsKey && !keySaved && !apiKeyInput.trim())}>
          {importing ? t.subForm.importing : t.subForm.add}
        </button>
      </form>

      {batchResult && (
        <div className="modal-backdrop batch-result-backdrop" onClick={() => setBatchResult(null)}>
          <div className="modal-container batch-result-modal" onClick={e => e.stopPropagation()}>
            <div className="batch-result-header">
              <h4 className="batch-result-title">{t.batchResult.title}</h4>
              <button className="vsm-close" onClick={() => setBatchResult(null)}>✕</button>
            </div>
            <div className="batch-result-body">
              {batchResult.succeeded.length > 0 && (
                <div className="batch-result-group success">
                  <span className="batch-result-label">{t.batchResult.success(batchResult.succeeded.length)}</span>
                  <span className="batch-result-symbols">{batchResult.succeeded.join(', ')}</span>
                </div>
              )}
              {batchResult.duplicates.length > 0 && (
                <div className="batch-result-group skipped">
                  <span className="batch-result-label">{t.batchResult.skipped(batchResult.duplicates.length)}</span>
                  <span className="batch-result-symbols">{batchResult.duplicates.join(', ')}</span>
                </div>
              )}
              {batchResult.failed.length > 0 && (
                <div className="batch-result-group failed">
                  <span className="batch-result-label">{t.batchResult.failed(batchResult.failed.length)}</span>
                  <span className="batch-result-symbols">{batchResult.failed.join(', ')}</span>
                </div>
              )}
              {batchResult.succeeded.length === 0 && batchResult.failed.length === 0 && (
                <div className="batch-result-group skipped">
                  <span className="batch-result-label">{t.batchResult.allExist}</span>
                </div>
              )}
            </div>
            <div className="batch-result-footer">
              <button className="view-editor-btn confirm" onClick={() => setBatchResult(null)}>{t.common.confirm}</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
