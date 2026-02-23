import { useState, useEffect } from 'react';
import { ProviderInfo, Subscription } from '../../types';
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

export function SubscriptionManager({ onBatchAdd, subscriptions, providers: providerInfoList, onToast }: SubscriptionManagerProps) {
  const [symbolInput, setSymbolInput] = useState('');
  const [assetType, setAssetType] = useState<'crypto' | 'stock'>('crypto');
  const [provider, setProvider] = useState('binance');
  const [importing, setImporting] = useState(false);
  const [importStatus, setImportStatus] = useState<{ done: number; total: number } | null>(null);
  const [batchResult, setBatchResult] = useState<BatchResult | null>(null);

  const filteredProviders = providerInfoList.filter(p =>
    assetType === 'crypto'
      ? (p.provider_type === 'crypto' || p.provider_type === 'both')
      : (p.provider_type === 'stock' || p.provider_type === 'both')
  );

  useEffect(() => {
    setProvider(assetType === 'crypto' ? 'binance' : 'yahoo');
  }, [assetType]);

  const selectedProviderInfo = providerInfoList.find(p => p.id === provider);
  const examples = selectedProviderInfo
    ? selectedProviderInfo.symbol_format.split(/[,，]\s*/).map(s => s.trim()).filter(Boolean)
    : [];

  const handleImport = async (e: React.FormEvent) => {
    e.preventDefault();
    const symbols = symbolInput
      .split(/[,\n\r;]+/)
      .map(s => s.trim())
      .filter(s => s.length > 0);
    if (symbols.length === 0) return;

    const existing = new Set(subscriptions.map(s => s.symbol.toUpperCase()));
    const unique = symbols.filter(s => !existing.has(s.toUpperCase()));
    const duplicates = symbols.filter(s => existing.has(s.toUpperCase()));

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
        onToast?.('已新增', `${unique[0].toUpperCase()} 訂閱成功`);
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
              <option key={p.id} value={p.id}>{p.name} {p.requires_api_key ? '(需API Key)' : ''}</option>
            ))}
          </select>
          {selectedProviderInfo && <span className="form-hint">{selectedProviderInfo.free_tier_info}</span>}
        </div>
        <div className="form-group">
          <label>代號</label>
          <input
            type="text"
            value={symbolInput}
            onChange={(e) => setSymbolInput(e.target.value)}
            placeholder={examples.length > 0 ? `例如: ${examples.join(', ')}` : '輸入代號'}
            required
            disabled={importing}
          />
          <span className="form-hint">支援多個代號，用逗號或分號分隔</span>
        </div>
        {importStatus && (
          <div className="batch-status">
            <span>匯入中 {importStatus.done} / {importStatus.total}</span>
          </div>
        )}
        <button type="submit" className="btn-add" disabled={!symbolInput.trim() || importing}>
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
