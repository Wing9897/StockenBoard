/**
 * LogoDownloader — 一鍵下載所有缺少 icon 的訂閱 logo（含進度條）
 */
import { useState, useEffect } from 'react';
import { getTransport } from '../../lib/transport';
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

export function LogoDownloader({ onToast }: Props) {
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState<ProgressPayload | null>(null);

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

  const pct = progress ? Math.round((progress.current / progress.total) * 100) : 0;

  return (
    <div className="settings-section">
      <h3>🖼️ {t.settings.downloadLogos}</h3>
      <p style={{ fontSize: '12px', color: 'var(--subtext0)', margin: '8px 0' }}>
        {t.settings.downloadLogosDesc}
      </p>
      <button
        className="dm-btn export"
        onClick={handleDownload}
        disabled={downloading}
      >
        {downloading ? t.settings.downloadLogosRunning : t.settings.downloadLogos}
      </button>
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
    </div>
  );
}
