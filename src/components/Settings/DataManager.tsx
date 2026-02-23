import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Subscription, View } from '../../types';
import { getDb } from '../../lib/db';
import './Settings.css';

interface DataManagerProps {
  subscriptions: Subscription[];
  views: View[];
  onRefresh: () => void;
  onToast?: (type: 'success' | 'error' | 'info', title: string, message?: string) => void;
}

interface ExportData {
  version: 1;
  exported_at: string;
  subscriptions: { symbol: string; display_name: string | null; provider: string; asset_type: string }[];
  views: { name: string; subscriptions: string[] }[];
}

export function DataManager({ subscriptions, views, onRefresh, onToast }: DataManagerProps) {
  const [importing, setImporting] = useState(false);
  const [importResult, setImportResult] = useState<{ subs: number; views: number; skipped: number } | null>(null);
  const [showExportPicker, setShowExportPicker] = useState(false);
  const [selectedViewIds, setSelectedViewIds] = useState<Set<number>>(new Set());

  const customViews = views.filter(v => !v.is_default);

  const openExportPicker = () => {
    // Default: select all custom views
    setSelectedViewIds(new Set(customViews.map(v => v.id)));
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
    const viewsToExport = customViews.filter(v => selectedViewIds.has(v.id));
    const viewExports: ExportData['views'] = [];

    for (const view of viewsToExport) {
      const rows = await db.select<{ subscription_id: number }[]>(
        'SELECT subscription_id FROM view_subscriptions WHERE view_id = $1',
        [view.id]
      );
      const syms = rows
        .map(r => subscriptions.find(s => s.id === r.subscription_id)?.symbol)
        .filter((s): s is string => !!s);
      viewExports.push({ name: view.name, subscriptions: syms });
    }

    // Export all subscriptions (they're always included for completeness)
    const data: ExportData = {
      version: 1,
      exported_at: new Date().toISOString(),
      subscriptions: subscriptions.map(s => ({
        symbol: s.symbol,
        display_name: s.display_name || null,
        provider: s.selected_provider_id,
        asset_type: s.asset_type,
      })),
      views: viewExports,
    };

    try {
      await invoke('export_file', {
        filename: `stockenboard_${new Date().toISOString().slice(0, 10)}.json`,
        content: JSON.stringify(data, null, 2),
      });
      setShowExportPicker(false);
      onToast?.('success', '匯出成功', '資料已儲存');
    } catch { /* cancelled */ }
  };

  const handleImport = async () => {
    let raw: string;
    try {
      raw = await invoke<string>('import_file');
    } catch { return; /* cancelled */ }

    let data: ExportData;
    try {
      data = JSON.parse(raw);
      if (!data.subscriptions || !Array.isArray(data.subscriptions)) throw new Error('invalid');
    } catch {
      onToast?.('error', '匯入失敗', '檔案格式不正確');
      return;
    }

    setImporting(true);
    const db = await getDb();
    const existing = new Set(subscriptions.map(s => s.symbol.toUpperCase()));
    let subsAdded = 0;
    let skipped = 0;

    // 用事務包裹批量 INSERT，減少 I/O 次數
    await db.execute('BEGIN TRANSACTION', []);
    for (const sub of data.subscriptions) {
      if (existing.has(sub.symbol.toUpperCase())) { skipped++; continue; }
      try {
        await db.execute(
          'INSERT INTO subscriptions (symbol, display_name, selected_provider_id, asset_type) VALUES ($1, $2, $3, $4)',
          [sub.symbol.toUpperCase(), sub.display_name || null, sub.provider, sub.asset_type || 'crypto']
        );
        existing.add(sub.symbol.toUpperCase());
        subsAdded++;
      } catch { skipped++; }
    }

    let viewsAdded = 0;
    if (data.views && Array.isArray(data.views)) {
      const existingViews = new Set(views.map(v => v.name.toLowerCase()));
      for (const v of data.views) {
        if (existingViews.has(v.name.toLowerCase())) { continue; }
        try {
          const result = await db.execute(
            'INSERT INTO views (name, is_default) VALUES ($1, 0)',
            [v.name]
          );
          const newViewId = result.lastInsertId;
          if (v.subscriptions && newViewId) {
            const allSubs = await db.select<{ id: number; symbol: string }[]>('SELECT id, symbol FROM subscriptions');
            const symMap = new Map(allSubs.map(s => [s.symbol.toUpperCase(), s.id]));
            for (const sym of v.subscriptions) {
              const subId = symMap.get(sym.toUpperCase());
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
      <h3>資料管理</h3>
      <div className="data-manager-actions">
        <button className="dm-btn export" onClick={openExportPicker} disabled={subscriptions.length === 0}>
          匯出資料
        </button>
        <button className="dm-btn import" onClick={handleImport} disabled={importing}>
          {importing ? '匯入中...' : '匯入資料'}
        </button>
      </div>
      {importResult && (
        <div className="dm-result">
          <span>匯入完成：新增 {importResult.subs} 訂閱、{importResult.views} 頁面</span>
          {importResult.skipped > 0 && <span>（跳過 {importResult.skipped} 已存在）</span>}
          <button className="dm-result-close" onClick={() => setImportResult(null)}>✕</button>
        </div>
      )}

      {showExportPicker && (
        <div className="dm-picker-backdrop" onClick={() => setShowExportPicker(false)}>
          <div className="dm-picker-modal" onClick={e => e.stopPropagation()}>
            <div className="dm-picker-header">
              <h4 className="dm-picker-title">選擇匯出頁面</h4>
              <button className="vsm-close" onClick={() => setShowExportPicker(false)}>✕</button>
            </div>
            <div className="dm-picker-info">
              所有訂閱 ({subscriptions.length}) 將自動包含
            </div>
            {customViews.length > 0 ? (
              <>
                <div className="dm-picker-actions">
                  <button className="dm-pick-btn" onClick={selectAll}>全選</button>
                  <button className="dm-pick-btn" onClick={selectNone}>取消全選</button>
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
                      </label>
                    </li>
                  ))}
                </ul>
              </>
            ) : (
              <div className="dm-picker-empty">沒有自訂頁面</div>
            )}
            <div className="dm-picker-footer">
              <span className="dm-picker-count">
                {selectedViewIds.size} / {customViews.length} 頁面
              </span>
              <button className="btn-save" onClick={handleExport}>
                匯出
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
