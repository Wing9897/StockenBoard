import { useState, useEffect, memo } from 'react';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { t } from '../../lib/i18n';

/**
 * 全域 icon blob 快取 — 同一個 iconName 只讀取一次檔案，
 * 轉成 blob URL 後所有卡片共用，不再重複走 asset:// 協議。
 */
const _blobCache = new Map<string, string>();        // iconName → blob URL
const _pendingLoads = new Map<string, Promise<string | null>>(); // 防止重複載入
const _failedIcons = new Set<string>();              // 確認不存在的 icon

let _iconsDirCache: string | null = null;
let _iconsDirPromise: Promise<string> | null = null;

function getIconsDir(): Promise<string> {
  if (_iconsDirCache) return Promise.resolve(_iconsDirCache);
  if (!_iconsDirPromise) {
    _iconsDirPromise = invoke<string>('get_icons_dir').then(dir => {
      _iconsDirCache = dir;
      return dir;
    });
  }
  return _iconsDirPromise;
}

/** 載入 icon 並轉成 blob URL，全域只執行一次 */
async function loadIconBlob(iconName: string): Promise<string | null> {
  if (_blobCache.has(iconName)) return _blobCache.get(iconName)!;
  if (_failedIcons.has(iconName)) return null;

  // 防止多張同 icon 的卡片同時觸發重複載入
  const pending = _pendingLoads.get(iconName);
  if (pending) return pending;

  const promise = (async () => {
    try {
      const dir = await getIconsDir();
      const sep = dir.endsWith('\\') || dir.endsWith('/') ? '' : '/';
      const assetUrl = convertFileSrc(`${dir}${sep}${iconName}.png`);

      const resp = await fetch(assetUrl);
      if (!resp.ok) throw new Error(`${resp.status}`);

      const blob = await resp.blob();
      const blobUrl = URL.createObjectURL(blob);
      _blobCache.set(iconName, blobUrl);
      return blobUrl;
    } catch {
      _failedIcons.add(iconName);
      return null;
    } finally {
      _pendingLoads.delete(iconName);
    }
  })();

  _pendingLoads.set(iconName, promise);
  return promise;
}

/** 清除特定 icon 的快取（set_icon 後呼叫） */
export function invalidateIcon(iconName: string) {
  const old = _blobCache.get(iconName);
  if (old) URL.revokeObjectURL(old);
  _blobCache.delete(iconName);
  _failedIcons.delete(iconName);
}

// ── 預載 icons dir ──
getIconsDir();

// ── Component ──

interface AssetIconProps {
  symbol: string;
  className: string;
  onClick: () => void;
}

export function getIconName(symbol: string): string {
  return symbol.toLowerCase().replace(/usdt$/, '').replace(/-usd$/, '');
}

export const AssetIcon = memo(function AssetIcon({ symbol, className, onClick }: AssetIconProps) {
  const iconName = getIconName(symbol);
  const fallbackText = iconName.slice(0, 3).toUpperCase();

  // 三態：null = 載入中/失敗, string = blob URL
  const [blobUrl, setBlobUrl] = useState<string | null>(() => _blobCache.get(iconName) ?? null);
  const [loaded, setLoaded] = useState(() => _blobCache.has(iconName));
  const [failed, setFailed] = useState(() => _failedIcons.has(iconName));

  useEffect(() => {
    // 已有快取 → 直接用
    if (_blobCache.has(iconName)) {
      setBlobUrl(_blobCache.get(iconName)!);
      setLoaded(true);
      setFailed(false);
      return;
    }
    if (_failedIcons.has(iconName)) {
      setFailed(true);
      setLoaded(true);
      return;
    }

    let cancelled = false;
    setLoaded(false);
    setFailed(false);

    loadIconBlob(iconName).then(url => {
      if (cancelled) return;
      if (url) {
        setBlobUrl(url);
        setLoaded(true);
      } else {
        setFailed(true);
        setLoaded(true);
      }
    });

    return () => { cancelled = true; };
  }, [iconName]);

  return (
    <div className={`${className} clickable`} onClick={onClick} title={t.asset.clickSetIcon}>
      {loaded && blobUrl && !failed ? (
        <img src={blobUrl} alt={symbol} />
      ) : (
        <span className="asset-icon-fallback">{fallbackText}</span>
      )}
    </div>
  );
});
