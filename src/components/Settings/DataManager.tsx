import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Subscription, View } from '../../types';
import { getDb } from '../../lib/db';
import { useConfirm } from '../../hooks/useConfirm';
import { useEscapeKey } from '../../hooks/useEscapeKey';
import { ConfirmDialog } from '../ConfirmDialog/ConfirmDialog';
import { t } from '../../lib/i18n';
import './Settings.css';

interface DataManagerProps {
  views: View[];
  onRefresh: () => void;
  onToast?: (type: 'success' | 'error' | 'info', title: string, message?: string) => void;
}

interface ExportData {
  version: 1;
  exported_at: string;
  subscriptions: {
    sub_type: 'asset' | 'dex';
    symbol: string;
    display_name: string | null;
    provider: string;
    asset_type: string;
    pool_address?: string;
    token_from_address?: string;
    token_to_address?: string;
  }[];
  views: { name: string; view_type: string; subscriptions: string[] }[];
}

export function DataManager({ views, onRefresh, onToast }: DataManagerProps) {
  const [importing, setImporting] = useState(false);
  const [importResult, setImportResult] = useState<{ subs: number; views: number; skipped: number } | null>(null);
  const [showExportPicker, setShowExportPicker] = useState(false);
  const [selectedViewIds, setSelectedViewIds] = useState<Set<number>>(new Set());
  const [allViews, setAllViews] = useState<View[]>([]);
  const { confirmState, requestConfirm, handleConfirm, handleCancel } = useConfirm();

  const customViews = allViews.filter(v => !v.is_default);

  useEscapeKey(() => { if (showExportPicker) setShowExportPicker(false); });

  const openExportPicker = async () => {
    try {
      const db = await getDb();
      const rows = await db.select<{ id: number; name: string; view_type: string; is_default: number }[]>(
        'SELECT id, name, view_type, is_default FROM views ORDER BY id'
      );
      const loaded = rows.map(r => ({ id: r.id, name: r.name, view_type: r.view_type as 'asset' | 'dex', is_default: r.is_default === 1 }));
      setAllViews(loaded);
      setSelectedViewIds(new Set(loaded.filter(v => !v.is_default).map(v => v.id)));
    } catch {
      setAllViews(views);
      setSelectedViewIds(new Set(views.filter(v => !v.is_default).map(v => v.id)));
    }
    setShowExportPicker(true);
  };

  const toggleView = (viewId: number) => {
    setSelectedViewIds(prev => {
      const next = new Set(prev);
      if (next.has(viewId)) next.delete(viewId);
      else next.add(viewId);
      return next;
    });
  };

  const selectAll = () => setSelectedViewIds(new Set(customViews.map(v => v.id)));
  const selectNone = () => setSelectedViewIds(new Set());

  const handleExport = async () => {
    const db = await getDb();

    const allSubs = await db.select<Subscription[]>(
      'SELECT id, sub_type, symbol, display_name, selected_provider_id, asset_type, pool_address, token_from_address, token_to_address, sort_order, record_enabled, record_from_hour, record_to_hour FROM subscriptions ORDER BY sort_order, id'
    );

    const viewsToExport = customViews.filter(v => selectedViewIds.has(v.id));
    const viewExports: ExportData['views'] = [];

    for (const view of viewsToExport) {
      const rows = await db.select<{ subscription_id: number }[]>(
        'SELECT subscription_id FROM view_subscriptions WHERE view_id = $1',
        [view.id]
      );
      const syms = rows
        .map(r => allSubs.find(s => s.id === r.subscription_id)?.symbol)
        .filter((s): s is string => !!s);
      viewExports.push({ name: view.name, view_type: view.view_type, subscriptions: syms });
    }

    const data: ExportData = {
      version: 1,
      exported_at: new Date().toISOString(),
      subscriptions: allSubs.map(s => ({
        sub_type: s.sub_type as 'asset' | 'dex',
        symbol: s.symbol,
        display_name: s.display_name || null,
        provider: s.selected_provider_id,
        asset_type: s.asset_type,
        ...(s.sub_type === 'dex' ? {
          pool_address: s.pool_address,
          token_from_address: s.token_from_address,
          token_to_address: s.token_to_address,
        } : {}),
      })),
      views: viewExports,
    };

    try {
      await invoke('export_file', {
        filename: `stockenboard_${new Date().toISOString().slice(0, 10)}.json`,
        content: JSON.stringify(data, null, 2),
      });
      setShowExportPicker(false);
      onToast?.('success', t.settings.exportSuccess, t.settings.exportSavedMsg);
    } catch { /* cancelled */ }
  };

  const handleImport = async () => {
    let raw: string;
    try {
      raw = await invoke<string>('import_file');
    } catch { return; }

    let data: ExportData;
    try {
      const parsed = JSON.parse(raw);
      if (!parsed.subscriptions || !Array.isArray(parsed.subscriptions)) throw new Error('invalid');
      data = {
        version: 1,
        exported_at: parsed.exported_at || '',
        subscriptions: parsed.subscriptions.map((s: ExportData['subscriptions'][0]) => ({
          sub_type: s.sub_type,
          symbol: s.symbol,
          display_name: s.display_name || null,
          provider: s.provider,
          asset_type: s.asset_type,
          pool_address: s.pool_address,
          token_from_address: s.token_from_address,
          token_to_address: s.token_to_address,
        })),
        views: (parsed.views || []).map((v: ExportData['views'][0]) => ({
          name: v.name,
          view_type: v.view_type,
          subscriptions: v.subscriptions || [],
        })),
      };
    } catch {
      onToast?.('error', t.settings.importFailed, t.settings.invalidFormat);
      return;
    }

    const confirmed = await requestConfirm(
      t.settings.importConfirm(data.subscriptions.length, data.views?.length || 0)
    );
    if (!confirmed) return;

    setImporting(true);
    const db = await getDb();
    const existingRows = await db.select<{ symbol: string; selected_provider_id: string }[]>(
      'SELECT symbol, selected_provider_id FROM subscriptions'
    );
    const existingKeys = new Set(existingRows.map(r => `${r.selected_provider_id}:${r.symbol}`));
    let subsAdded = 0;
    let skipped = 0;

    await db.execute('BEGIN TRANSACTION', []);
    for (const sub of data.subscriptions) {
      const isDex = sub.sub_type === 'dex';
      const storedSymbol = isDex ? sub.symbol : sub.symbol.toUpperCase();
      const key = `${sub.provider}:${storedSymbol}`;
      if (existingKeys.has(key)) { skipped++; continue; }
      try {
        if (isDex) {
          await db.execute(
            'INSERT INTO subscriptions (sub_type, symbol, display_name, selected_provider_id, asset_type, pool_address, token_from_address, token_to_address) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)',
            [
              'dex', storedSymbol, sub.display_name || null, sub.provider, sub.asset_type || 'crypto',
              sub.pool_address || null, sub.token_from_address || null, sub.token_to_address || null,
            ]
          );
        } else {
          await db.execute(
            'INSERT INTO subscriptions (sub_type, symbol, display_name, selected_provider_id, asset_type) VALUES ($1, $2, $3, $4, $5)',
            ['asset', storedSymbol, sub.display_name || null, sub.provider, sub.asset_type || 'crypto']
          );
        }
        existingKeys.add(key);
        subsAdded++;
      } catch { skipped++; }
    }

    let viewsAdded = 0;
    if (data.views && Array.isArray(data.views)) {
      const existingViewRows = await db.select<{ name: string; view_type: string }[]>(
        'SELECT name, view_type FROM views'
      );
      const existingViews = new Set(existingViewRows.map(v => `${v.view_type}:${v.name.toLowerCase()}`));
      for (const v of data.views) {
        const viewType = v.view_type || 'asset';
        if (existingViews.has(`${viewType}:${v.name.toLowerCase()}`)) { continue; }
        try {
          const result = await db.execute(
            'INSERT INTO views (name, view_type, is_default) VALUES ($1, $2, 0)',
            [v.name, viewType]
          );
          const newViewId = result.lastInsertId;
          if (v.subscriptions && newViewId) {
            const allSubs = await db.select<{ id: number; symbol: string }[]>('SELECT id, symbol FROM subscriptions');
            const symMapExact = new Map(allSubs.map(s => [s.symbol, s.id]));
            const symMapUpper = new Map(allSubs.map(s => [s.symbol.toUpperCase(), s.id]));
            for (const sym of v.subscriptions) {
              const subId = symMapExact.get(sym) ?? symMapUpper.get(sym.toUpperCase());
              if (subId) {
                await db.execute(
                  'INSERT OR IGNORE INTO view_subscriptions (view_id, subscription_id) VALUES ($1, $2)',
                  [newViewId, subId]
                );
              }
            }
          }
          viewsAdded++;
        } catch { /* skip */ }
      }
    }
    await db.execute('COMMIT', []);

    setImporting(false);
    setImportResult({ subs: subsAdded, views: viewsAdded, skipped });
    onRefresh();
  };

  return (
    <div className="settings-section">
      <h3>{t.settings.dataManager}</h3>
      <div className="data-manager-actions">
        <button className="dm-btn export" onClick={openExportPicker}>
          {t.settings.export}
        </button>
        <button className="dm-btn import" onClick={handleImport} disabled={importing}>
          {importing ? t.settings.importing : t.settings.import}
        </button>
      </div>
      {importResult && (
        <div className="dm-result">
          <span>{t.settings.importDone(importResult.subs, importResult.views)}</span>
          {importResult.skipped > 0 && <span>{t.settings.importSkipped(importResult.skipped)}</span>}
          <button className="dm-result-close" onClick={() => setImportResult(null)} aria-label={t.common.close}>✕</button>
        </div>
      )}

      {showExportPicker && (
        <div className="modal-backdrop dm-picker-backdrop" onClick={() => setShowExportPicker(false)}>
          <div className="modal-container dm-picker-modal" role="dialog" aria-modal="true" aria-label={t.settings.exportPicker} onClick={e => e.stopPropagation()}>
            <div className="dm-picker-header">
              <h4 className="dm-picker-title">{t.settings.exportPicker}</h4>
              <button className="vsm-close" onClick={() => setShowExportPicker(false)} aria-label={t.common.close}>✕</button>
            </div>
            <div className="dm-picker-info">
              {t.settings.exportPickerInfo}
            </div>
            {customViews.length > 0 ? (
              <>
                <div className="dm-picker-actions">
                  <button className="dm-pick-btn" onClick={selectAll}>{t.subs.selectAll}</button>
                  <button className="dm-pick-btn" onClick={selectNone}>{t.subs.clearAll}</button>
                </div>
                <ul className="dm-picker-list">
                  {customViews.map(view => (
                    <li key={view.id} className="dm-picker-item">
                      <label className="dm-picker-label">
                        <input
                          type="checkbox"
                          checked={selectedViewIds.has(view.id)}
                          onChange={() => toggleView(view.id)}
                        />
                        <span>{view.name}</span>
                        <span className={`asset-type-tag ${view.view_type} dm-picker-tag`}>
                          {view.view_type === 'dex' ? 'DEX' : t.settings.spot}
                        </span>
                      </label>
                    </li>
                  ))}
                </ul>
              </>
            ) : (
              <div className="dm-picker-empty">{t.settings.noCustomViews}</div>
            )}
            <div className="dm-picker-footer">
              <span className="dm-picker-count">
                {t.settings.viewCount(selectedViewIds.size, customViews.length)}
              </span>
              <button className="btn-save" onClick={handleExport}>
                {t.settings.export}
              </button>
            </div>
          </div>
        </div>
      )}

      {confirmState && (
        <ConfirmDialog message={confirmState.message} onConfirm={handleConfirm} onCancel={handleCancel} />
      )}
    </div>
  );
}
