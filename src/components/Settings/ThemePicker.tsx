import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { t } from '../../lib/i18n';
import './Settings.css';

type ThemeEntry = {
  id: string;
  name: string;
  colors: string[];
  anime?: boolean;
  gradient?: string;
};

const THEMES: ThemeEntry[] = [
  { id: 'mocha', name: 'Mocha', colors: ['#1e1e2e', '#313244', '#89b4fa', '#a6e3a1', '#cdd6f4'] },
  { id: 'macchiato', name: 'Macchiato', colors: ['#24273a', '#363a4f', '#8aadf4', '#a6da95', '#cad3f5'] },
  { id: 'frappe', name: 'Frappé', colors: ['#303446', '#414559', '#8caaee', '#a6d189', '#c6d0f5'] },
  { id: 'latte', name: 'Latte', colors: ['#eff1f5', '#ccd0da', '#1e66f5', '#40a02b', '#4c4f69'] },
  { id: 'midnight', name: 'Midnight', colors: ['#0d0d0d', '#1a1a1a', '#6ea8fe', '#75d47a', '#e0e0e0'] },
  { id: 'nord', name: 'Nord', colors: ['#2e3440', '#3b4252', '#88c0d0', '#a3be8c', '#eceff4'] },
  { id: 'dracula', name: 'Dracula', colors: ['#282a36', '#44475a', '#bd93f9', '#50fa7b', '#f8f8f2'] },
  { id: 'gruvbox', name: 'Gruvbox', colors: ['#282828', '#3c3836', '#fabd2f', '#b8bb26', '#ebdbb2'] },
  { id: 'sakura', name: '🌸 Sakura', anime: true, colors: ['#2d1423', '#f48fb1', '#ce93d8', '#fce4ec', '#90caf9'], gradient: 'linear-gradient(135deg, #2d1423, #3d1a30, #2a1028)' },
  { id: 'cyberpunk', name: '⚡ Cyberpunk', anime: true, colors: ['#050510', '#00d4ff', '#bb66ff', '#ff3366', '#00ff88'], gradient: 'linear-gradient(160deg, #050510, #0a0a1e, #0f0828)' },
  { id: 'ghibli', name: '🌿 Ghibli', anime: true, colors: ['#0a1a10', '#a5d6a7', '#81d4fa', '#fff9c4', '#e8f5e9'], gradient: 'linear-gradient(150deg, #0a1a10, #142518, #1a3020)' },
  { id: 'evangelion', name: '🤖 EVA', anime: true, colors: ['#0a0018', '#9b59b6', '#2ecc71', '#e67e22', '#e0d0ff'], gradient: 'linear-gradient(145deg, #0a0018, #1a0030, #0d0020)' },
  { id: 'miku', name: '🎵 Miku', anime: true, colors: ['#0a1820', '#39c5bb', '#e12885', '#86cecb', '#e0f7f6'], gradient: 'linear-gradient(140deg, #0a1820, #0d2028, #081a22)' },
  { id: 'sunset', name: '🌅 Sunset', anime: true, colors: ['#1a0a10', '#ff6b35', '#9b59b6', '#ffd700', '#ffe0d0'], gradient: 'linear-gradient(155deg, #1a0a10, #2a1018, #1a0820)' },
];

/** Anime theme IDs — 也被 main.tsx 用到 */
export const ANIME_IDS = new Set(THEMES.filter(t => t.anime).map(t => t.id));

/** Apply background image + opacity to the DOM — 也被 main.tsx 使用 */
export function applyBgImage(url: string | null, opacity: number) {
  let el = document.getElementById('sb-theme-bg');
  if (!url) {
    if (el) el.style.display = 'none';
    return;
  }
  if (!el) {
    el = document.createElement('div');
    el.id = 'sb-theme-bg';
    Object.assign(el.style, {
      position: 'fixed', inset: '0', zIndex: '0', pointerEvents: 'none',
      backgroundSize: 'cover', backgroundPosition: 'center', backgroundRepeat: 'no-repeat',
    });
    document.body.prepend(el);
  }
  el.style.display = 'block';
  el.style.backgroundImage = `url("${url}")`;
  el.style.opacity = String(opacity);
}

/** 從 Rust 載入 bg 並套用到 DOM，回傳 data URL 或 null — 也被 main.tsx 使用 */
export async function loadBgForTheme(themeId: string): Promise<string | null> {
  if (!ANIME_IDS.has(themeId)) {
    applyBgImage(null, 0);
    return null;
  }
  try {
    const filePath = await invoke<string | null>('get_theme_bg_path', { themeId });
    if (filePath) {
      const dataUrl = await invoke<string>('read_local_file_base64', { path: filePath });
      const opacity = parseFloat(localStorage.getItem(`sb_theme_bg_opacity_${themeId}`) || '0.3');
      applyBgImage(dataUrl, opacity);
      return dataUrl;
    }
  } catch { /* ignore */ }
  applyBgImage(null, 0);
  return null;
}

export function ThemePicker() {
  const [current, setCurrent] = useState(() => localStorage.getItem('sb_theme') || 'mocha');
  const [bgUrl, setBgUrl] = useState<string | null>(null);
  const [bgOpacity, setBgOpacity] = useState(0.3);

  const isAnime = ANIME_IDS.has(current);

  // Load bg on mount & theme change
  useEffect(() => {
    let cancelled = false;
    const handler = async () => {
      const url = await loadBgForTheme(localStorage.getItem('sb_theme') || 'mocha');
      if (!cancelled) {
        setCurrent(localStorage.getItem('sb_theme') || 'mocha');
        setBgOpacity(parseFloat(localStorage.getItem(`sb_theme_bg_opacity_${localStorage.getItem('sb_theme') || 'mocha'}`) || '0.3'));
        setBgUrl(url);
      }
    };
    handler();
    window.addEventListener('theme-change', handler);
    return () => { cancelled = true; window.removeEventListener('theme-change', handler); };
  }, []);

  const handleSelect = async (id: string) => {
    document.documentElement.setAttribute('data-theme', id);
    localStorage.setItem('sb_theme', id);
    setCurrent(id);
    setBgOpacity(parseFloat(localStorage.getItem(`sb_theme_bg_opacity_${id}`) || '0.3'));
    const url = await loadBgForTheme(id);
    setBgUrl(url);
    window.dispatchEvent(new Event('theme-change'));
  };

  const handleChooseBg = async () => {
    try {
      const filePath = await invoke<string>('save_theme_bg', { themeId: current });
      const dataUrl = await invoke<string>('read_local_file_base64', { path: filePath });
      setBgUrl(dataUrl);
      applyBgImage(dataUrl, bgOpacity);
    } catch { /* cancelled */ }
  };

  const handleRemoveBg = async () => {
    try { await invoke('remove_theme_bg', { themeId: current }); } catch { /* ignore */ }
    setBgUrl(null);
    applyBgImage(null, 0);
  };

  const handleOpacityChange = (val: number) => {
    setBgOpacity(val);
    localStorage.setItem(`sb_theme_bg_opacity_${current}`, String(val));
    if (bgUrl) applyBgImage(bgUrl, val);
  };

  return (
    <div className="settings-section theme-section">
      <h3>{t.settings.theme}</h3>
      <div className="theme-grid">
        {THEMES.map(th => (
          <div
            key={th.id}
            className={`theme-card ${current === th.id ? 'active' : ''} ${th.anime ? 'anime' : ''}`}
            onClick={() => handleSelect(th.id)}
            role="button"
            aria-pressed={current === th.id}
            aria-label={th.name}
            tabIndex={0}
            onKeyDown={e => { if (e.key === 'Enter' || e.key === ' ') handleSelect(th.id); }}
          >
            <div className="theme-preview" style={th.gradient ? { background: th.gradient } : undefined}>
              {th.gradient
                ? th.colors.slice(1).map((c, i) => (
                    <span key={i} style={{ background: c, borderRadius: '50%', width: 14, height: 14, flex: 'none', boxShadow: `0 0 6px ${c}` }} />
                  ))
                : th.colors.map((c, i) => (
                    <span key={i} style={{ background: c }} />
                  ))
              }
            </div>
            <span className="theme-label">{th.name}</span>
          </div>
        ))}
      </div>

      {isAnime && (
        <div className="theme-bg-section">
          <h4>{t.settings.bgImage}</h4>
          <div className="theme-bg-controls">
            <button className="theme-bg-btn choose" onClick={handleChooseBg} aria-label={t.settings.chooseBg}>
              {t.settings.chooseBg}
            </button>
            {bgUrl && (
              <button className="theme-bg-btn remove" onClick={handleRemoveBg} aria-label={t.settings.removeBg}>
                {t.settings.removeBg}
              </button>
            )}
          </div>
          {bgUrl && (
            <>
              <div className="theme-bg-preview-wrap">
                <img src={bgUrl} alt="bg" className="theme-bg-preview" />
              </div>
              <div className="theme-bg-opacity">
                <label>{t.settings.bgOpacity}</label>
                <input
                  type="range"
                  min="0.05"
                  max="0.6"
                  step="0.05"
                  value={bgOpacity}
                  onChange={e => handleOpacityChange(parseFloat(e.target.value))}
                />
                <span>{Math.round(bgOpacity * 100)}%</span>
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}
