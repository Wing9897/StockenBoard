import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { View } from '../../types';
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

// Rust 端匯出格式
interface RustExportData {
  subscriptions: {
    symbol: string;
    display_name: string | null;
    selected_provider_id: string;
    asset_type: string;
    sub_type: string;
    pool_address: string | null;
    token_from_address: string | null;
    token_to_address: string | null;
  }[];
  views: { name: string; view_type: string; symbols: string[] }[];
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
      const assetViews = await invoke<{ id: number; name: string; view_type: string; is_default: boolean }[]>('list_views', { viewType: 'asset' });
      const dexViews = await invoke<{ id: number; name: string; view_type: string; is_default: boolean }[]>('list_views', { viewType: 'dex' });
      const loaded = [...assetViews, ...dexViews].map(r => ({
        id: r.id, name: r.name, view_type: r.view_type as 'asset' | 'dex', is_default: r.is_default
      }));
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
    // 使用 Rust 端匯出，然後在前端格式化
    try {
      const rustData = await invoke<RustExportData>('export_data');
      const data: ExportData = {
        version: 1,
        exported_at: new Date().toISOString(),
        subscriptions: rustData.subscriptions.map(s => ({
          sub_type: s.sub_type as 'asset' | 'dex',
          symbol: s.symbol,
          display_name: s.display_name,
          provider: s.selected_provider_id,
          asset_type: s.asset_type,
          ...(s.sub_type === 'dex' ? {
            pool_address: s.pool_address || undefined,
            token_from_address: s.token_from_address || undefined,
            token_to_address: s.token_to_address || undefined,
          } : {}),
        })),
        views: rustData.views
          .filter(v => {
            // 找到對應的 view ID 來判斷是否被選取
            const matchedView = allViews.find(av => av.name === v.name && av.view_type === v.view_type);
            return matchedView ? selectedViewIds.has(matchedView.id) : false;
          })
          .map(v => ({ name: v.name, view_type: v.view_type, subscriptions: v.symbols })),
      };

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

    // 轉換為 Rust 端格式，透過 IPC 匯入
    const rustImportData: RustExportData = {
      subscriptions: data.subscriptions.map(s => ({
        symbol: s.sub_type === 'dex' ? s.symbol : s.symbol.toUpperCase(),
        display_name: s.display_name,
        selected_provider_id: s.provider,
        asset_type: s.asset_type || 'crypto',
        sub_type: s.sub_type || 'asset',
        pool_address: s.pool_address || null,
        token_from_address: s.token_from_address || null,
        token_to_address: s.token_to_address || null,
      })),
      views: (data.views || []).map(v => ({
        name: v.name,
        view_type: v.view_type || 'asset',
        symbols: v.subscriptions || [],
      })),
    };

    try {
      const [imported, skipped] = await invoke<[number, number]>('import_data', { data: rustImportData });
      setImportResult({ subs: imported, views: data.views?.length || 0, skipped });
    } catch (e) {
      onToast?.('error', t.settings.importFailed, String(e));
    }

    setImporting(false);
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
