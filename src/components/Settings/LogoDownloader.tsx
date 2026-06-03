/**
 * LogoDownloader — 一鍵下載所有缺少 icon 的訂閱 logo
 */
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { t } from '../../lib/i18n';
import { clearAllIcons } from '../AssetCard/AssetIcon';

interface LogoDownloadResult {
  succeeded: number;
  skipped: number;
  failed: number;
  failed_symbols: string[];
}

interface Props {
  onToast?: (type: 'success' | 'error' | 'info', title: string, message?: string) => void;
}

export function LogoDownloader({ onToast }: Props) {
  const [downloading, setDownloading] = useState(false);

  const handleDownload = async () => {
    setDownloading(true);
    try {
      const result = await invoke<LogoDownloadResult>('download_logos');
      // 清除 icon 快取，讓新下載的 logo 馬上顯示
      clearAllIcons();
      onToast?.('success', t.settings.downloadLogos,
        t.settings.downloadLogosDone(result.succeeded, result.skipped, result.failed));
    } catch (e) {
      onToast?.('error', t.settings.downloadLogos, typeof e === 'string' ? e : String(e));
    } finally {
      setDownloading(false);
    }
  };

  return (
    <div className="ps-section" style={{ padding: '16px 20px' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '12px' }}>
        <div>
          <div style={{ fontSize: '14px', fontWeight: 500, color: 'var(--text)', marginBottom: '4px' }}>
            🖼️ {t.settings.downloadLogos}
          </div>
          <div style={{ fontSize: '12px', color: 'var(--subtext0)' }}>
            {t.settings.downloadLogosDesc}
          </div>
        </div>
        <button
          className="dm-btn export"
          onClick={handleDownload}
          disabled={downloading}
          style={{ flexShrink: 0 }}
        >
          {downloading ? t.settings.downloadLogosRunning : t.settings.downloadLogos}
        </button>
      </div>
    </div>
  );
}
