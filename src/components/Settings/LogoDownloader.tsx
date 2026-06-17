/**
 * LogoDownloader — 一鍵下載所有缺少 icon 的訂閱 logo + 手動搜索下載
 */
import { useState, useEffect } from 'react';
import { getTransport, isTauri } from '../../lib/transport';
import { t } from '../../lib/i18n';
import { clearAllIcons } from '../AssetCard/AssetIcon';

interface LogoDownloadResult {
  succeeded: number;
  skipped: number;
  failed: number;
  failed_symbols: string[];
}

interface ProgressPayload {
  current: number;
  total: number;
  symbol: string;
}

interface Props {
  onToast?: (type: 'success' | 'error' | 'info', title: string, message?: string) => void;
}

function SearchIconModal({ onClose, onToast }: { onClose: () => void; onToast?: Props['onToast'] }) {
  const [symbol, setSymbol] = useState('');
  const [saveAs, setSaveAs] = useState('');
  const [results, setResults] = useState<{ source: string; data: string }[]>([]);
  const [selectedIdx, setSelectedIdx] = useState<number | null>(null);
  const [searching, setSearching] = useState(false);
  const [saving, setSaving] = useState(false);

  const handleSearch = async () => {
    if (!symbol.trim()) return;
    setSearching(true);
    setResults([]);
    setSelectedIdx(null);
    if (!saveAs) setSaveAs(symbol.trim().toLowerCase());
    try {
      const items = await getTransport().invoke<{ source: string; data: string }[]>('search_icons', { symbol: symbol.trim() });
      setResults(items);
      if (items.length > 0) setSelectedIdx(0);
    } catch {
      setResults([]);
    } finally {
      setSearching(false);
    }
  };

  const handleSave = async () => {
    if (selectedIdx === null || !saveAs.trim()) return;
    const selected = results[selectedIdx];
    setSaving(true);
    try {
      await getTransport().invoke('save_icon_from_data', {
        saveAs: saveAs.trim().toLowerCase(),
        dataUrl: selected.data,
      });
      clearAllIcons();
      onToast?.('success', t.settings.logoManagement, `${saveAs.trim().toLowerCase()}.png ✓`);
      onClose();
    } catch (e) {
      onToast?.('error', t.settings.logoManagement, typeof e === 'string' ? e : String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="logo-search-modal" onClick={e => e.stopPropagation()}>
        <div className="logo-search-header">
          <h3>{t.settings.searchIcon}</h3>
          <button className="btn-close" onClick={onClose}>✕</button>
        </div>
        <div className="logo-search-body">
          <div className="logo-search-input-row">
            <input
              type="text"
              value={symbol}
              onChange={e => setSymbol(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && handleSearch()}
              placeholder={t.settings.logoSymbolPlaceholder}
              autoFocus
            />
            <button className="btn-save" onClick={handleSearch} disabled={searching || !symbol.trim()}>
              {searching ? '...' : t.settings.searchIcon}
            </button>
          </div>

          {results.length > 0 && (
            <div className="logo-search-results">
              {results.map((r, i) => (
                <div
                  key={i}
                  className={`logo-search-item ${selectedIdx === i ? 'selected' : ''}`}
                  onClick={() => setSelectedIdx(i)}
                >
                  <img src={r.data} alt={r.source} width={36} height={36} />
                  <span>{r.source}</span>
                </div>
              ))}
            </div>
          )}
          {!searching && results.length === 0 && symbol.trim() && (
            <p className="logo-preview-error">{t.settings.logoNotFound}</p>
          )}

          {selectedIdx !== null && (
            <label className="form-field">
              <span>{t.settings.logoSaveAsPlaceholder}</span>
              <input
                type="text"
                value={saveAs}
                onChange={e => setSaveAs(e.target.value)}
                placeholder="btc"
              />
            </label>
          )}
        </div>
        <div className="logo-search-actions">
          <button className="btn-cancel" onClick={onClose}>{t.common.cancel}</button>
          <button className="btn-save" onClick={handleSave} disabled={selectedIdx === null || !saveAs.trim() || saving}>
            {saving ? t.common.saving : t.common.save}
          </button>
        </div>
      </div>
    </div>
  );
}

export function LogoDownloader({ onToast }: Props) {
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState<ProgressPayload | null>(null);
  const [iconsPath, setIconsPath] = useState<string | null>(null);
  const [showSearchModal, setShowSearchModal] = useState(false);

  useEffect(() => {
    const unlisten = getTransport().listen('logo-download-progress', (payload) => {
      setProgress(payload as ProgressPayload);
    });
    return () => { unlisten(); };
  }, []);

  const handleDownload = async () => {
    setDownloading(true);
    setProgress(null);
    try {
      const result = await getTransport().invoke<LogoDownloadResult>('download_logos');
      clearAllIcons();
      onToast?.('success', t.settings.downloadLogos,
        t.settings.downloadLogosDone(result.succeeded, result.skipped, result.failed));
    } catch (e) {
      onToast?.('error', t.settings.downloadLogos, typeof e === 'string' ? e : String(e));
    } finally {
      setDownloading(false);
      setProgress(null);
    }
  };

  const handleOpenFolder = async () => {
    if (isTauri()) {
      try {
        await getTransport().invoke('open_icons_folder');
      } catch (e) {
        onToast?.('error', t.settings.logoManagement, typeof e === 'string' ? e : String(e));
      }
    } else {
      try {
        const dir = await getTransport().invoke<string>('get_icons_dir');
        setIconsPath(dir);
      } catch (e) {
        onToast?.('error', t.settings.logoManagement, typeof e === 'string' ? e : String(e));
      }
    }
  };

  const handleClearAll = async () => {
    try {
      const n = await getTransport().invoke<number>('clear_all_icons');
      clearAllIcons();
      onToast?.('success', t.settings.logoManagement, t.settings.clearLogosDone(n));
    } catch (e) {
      onToast?.('error', t.settings.logoManagement, typeof e === 'string' ? e : String(e));
    }
  };

  const pct = progress ? Math.round((progress.current / progress.total) * 100) : 0;

  return (
    <div className="settings-section">
      <h3>🖼️ {t.settings.logoManagement}</h3>
      <p style={{ fontSize: '12px', color: 'var(--subtext0)', margin: '8px 0' }}>
        {t.settings.downloadLogosDesc}
      </p>

      <div className="data-manager-actions">
        <button className="dm-btn export" onClick={handleDownload} disabled={downloading}>
          {downloading ? t.settings.downloadLogosRunning : t.settings.downloadLogos}
        </button>
        <button className="dm-btn import" onClick={() => setShowSearchModal(true)}>
          {t.settings.searchIcon}
        </button>
        <button className="dm-btn export" onClick={handleOpenFolder}>
          {t.settings.openFolder}
        </button>
        <button className="dm-btn danger" onClick={handleClearAll}>
          {t.settings.clearLogos}
        </button>
      </div>

      {downloading && progress && (
        <div style={{ marginTop: '10px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', color: 'var(--subtext0)', marginBottom: '4px' }}>
            <span>{progress.symbol}</span>
            <span>{progress.current}/{progress.total} ({pct}%)</span>
          </div>
          <div style={{ width: '100%', height: '4px', background: 'var(--surface0)', borderRadius: '2px', overflow: 'hidden' }}>
            <div style={{ width: `${pct}%`, height: '100%', background: 'var(--blue)', borderRadius: '2px', transition: 'width 0.2s ease' }} />
          </div>
        </div>
      )}

      {iconsPath && (
        <p style={{ fontSize: '11px', color: 'var(--subtext0)', margin: '6px 0 0', wordBreak: 'break-all' }}>
          📁 {iconsPath}
        </p>
      )}

      {showSearchModal && <SearchIconModal onClose={() => setShowSearchModal(false)} onToast={onToast} />}
    </div>
  );
}
