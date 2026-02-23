import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ProviderInfo, Subscription } from '../../types';
import { getDb } from '../../lib/db';
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
  onToast?: (title: string, message?: string) => void;
}

/** 查詢某 provider 是否已存有 API key */
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

/** 儲存 API key 到 provider_settings 並同步 Rust 端 */
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

  // API key 相關
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

  // 切換 provider 時檢查是否已有 key
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
      onToast?.('API Key 已儲存', `${selectedProviderInfo?.name} 的 API Key 已設定`);
    } catch (err) {
      onToast?.('儲存失敗', err instanceof Error ? err.message : String(err));
    } finally {
      setKeySaving(false);
    }
  };

  const handleImport = async (e: React.FormEvent) => {
    e.preventDefault();

    // 如果需要 key 但還沒儲存，先儲存
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

    // Single symbol → toast instead of modal
    if (symbols.length === 1) {
      if (duplicates.length === 1) {
        onToast?.('已存在', `${duplicates[0]} 已訂閱`);
        return;
      }
      setImporting(true);
      setImportStatus({ done: 0, total: 1 });
      try {
        await onBatchAdd(unique[0], provider, assetType);
        onToast?.('已新增', `${isDex ? unique[0] : unique[0].toUpperCase()} 訂閱成功`);
        setSymbolInput('');
      } catch {
        onToast?.('新增失敗', `${unique[0]} 無法新增`);
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
      <h3>新增訂閱</h3>

      <form className="subscription-form" onSubmit={handleImport}>
        <div className="form-group">
          <label>資產類型</label>
          <div className="asset-type-toggle">
            <button type="button" className={`type-btn ${assetType === 'crypto' ? 'active' : ''}`} onClick={() => setAssetType('crypto')}>加密貨幣</button>
            <button type="button" className={`type-btn ${assetType === 'stock' ? 'active' : ''}`} onClick={() => setAssetType('stock')}>股票</button>
          </div>
        </div>
        <div className="form-group">
          <label>默認數據源</label>
          <select value={provider} onChange={(e) => setProvider(e.target.value)}>
            {filteredProviders.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name} {p.requires_api_key ? '(需API Key)' : p.optional_api_key ? '(可選Key)' : ''}
              </option>
            ))}
          </select>
          {selectedProviderInfo && <span className="form-hint">{selectedProviderInfo.free_tier_info}</span>}
        </div>

        {showKeyInput && (
          <div className="form-group api-key-group">
            <label>
              API Key
              {optionalKey && !needsKey && <span className="optional-tag">可選，提高速率</span>}
              {needsKey && <span className="required-tag">必填</span>}
              {keySaved && <span className="saved-tag">✓ 已儲存</span>}
            </label>
            {keySaved ? (
              <div className="key-saved-row">
                <span className="key-saved-text">已設定 API Key</span>
                <button type="button" className="btn-change-key" onClick={() => { setKeySaved(false); setApiKeyInput(''); setApiSecretInput(''); }}>
                  更換
                </button>
              </div>
            ) : (
              <>
                <input
                  type="password"
                  value={apiKeyInput}
                  onChange={(e) => setApiKeyInput(e.target.value)}
                  placeholder="輸入 API Key"
                  disabled={importing || keySaving}
                />
                {needsSecret && (
                  <input
                    type="password"
                    value={apiSecretInput}
                    onChange={(e) => setApiSecretInput(e.target.value)}
                    placeholder="輸入 API Secret"
                    disabled={importing || keySaving}
                    style={{ marginTop: '4px' }}
                  />
                )}
                {apiKeyInput.trim() && (
                  <button type="button" className="btn-save-key" onClick={handleSaveKey} disabled={keySaving}>
                    {keySaving ? '儲存中...' : '儲存 Key'}
                  </button>
                )}
              </>
            )}
          </div>
        )}

        <div className="form-group">
          <label>{isDex ? '代號 / 合約地址' : '代號'}</label>
          {isDex ? (
            <textarea
              value={symbolInput}
              onChange={(e) => setSymbolInput(e.target.value)}
              placeholder={examples.length > 0 ? `例如: ${examples.join(', ')}` : '輸入代號或合約地址'}
              required
              disabled={importing}
              rows={3}
            />
          ) : (
            <input
              type="text"
              value={symbolInput}
              onChange={(e) => setSymbolInput(e.target.value)}
              placeholder={examples.length > 0 ? `例如: ${examples.join(', ')}` : '輸入代號'}
              required
              disabled={importing}
            />
          )}
          {isDex && provider === 'jupiter' && (
            <span className="form-hint">
              支援常見代號 (SOL, JUP, BONK, WIF) 或 Solana mint address
            </span>
          )}
          {isDex && provider === 'okx_dex' && (
            <span className="form-hint">
              支援常見代號 (ETH, BNB, SOL) 或「鏈:合約地址」格式，如 eth:0x..., sol:mint_address, arb:0x...
            </span>
          )}
          {!isDex && <span className="form-hint">支援多個代號，用逗號或分號分隔</span>}
        </div>
        {importStatus && (
          <div className="batch-status">
            <span>匯入中 {importStatus.done} / {importStatus.total}</span>
          </div>
        )}
        <button type="submit" className="btn-add" disabled={!symbolInput.trim() || importing || (needsKey && !keySaved && !apiKeyInput.trim())}>
          {importing ? '匯入中...' : '新增'}
        </button>
      </form>

      {batchResult && (
        <div className="batch-result-backdrop" onClick={() => setBatchResult(null)}>
          <div className="batch-result-modal" onClick={e => e.stopPropagation()}>
            <div className="batch-result-header">
              <h4 className="batch-result-title">匯入結果</h4>
              <button className="vsm-close" onClick={() => setBatchResult(null)}>✕</button>
            </div>
            <div className="batch-result-body">
              {batchResult.succeeded.length > 0 && (
                <div className="batch-result-group success">
                  <span className="batch-result-label">✓ 成功 ({batchResult.succeeded.length})</span>
                  <span className="batch-result-symbols">{batchResult.succeeded.join(', ')}</span>
                </div>
              )}
              {batchResult.duplicates.length > 0 && (
                <div className="batch-result-group skipped">
                  <span className="batch-result-label">⊘ 已存在跳過 ({batchResult.duplicates.length})</span>
                  <span className="batch-result-symbols">{batchResult.duplicates.join(', ')}</span>
                </div>
              )}
              {batchResult.failed.length > 0 && (
                <div className="batch-result-group failed">
                  <span className="batch-result-label">✗ 失敗 ({batchResult.failed.length})</span>
                  <span className="batch-result-symbols">{batchResult.failed.join(', ')}</span>
                </div>
              )}
              {batchResult.succeeded.length === 0 && batchResult.failed.length === 0 && (
                <div className="batch-result-group skipped">
                  <span className="batch-result-label">所有代號都已存在</span>
                </div>
              )}
            </div>
            <div className="batch-result-footer">
              <button className="view-editor-btn confirm" onClick={() => setBatchResult(null)}>確定</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
