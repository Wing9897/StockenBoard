import { useState, useEffect, useRef } from 'react';
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
  { id: 'sakura', name: '🌸 Sakura', anime: true, colors: ['#2d1423', '#f48fb1', '#ce93d8', '#fce4ec', '#90caf9'], gradient: 'linear-gradient(135deg, #2d1423, #3d1a30, #2a1028)' },
  { id: 'cyberpunk', name: '⚡ Cyberpunk', anime: true, colors: ['#050510', '#00d4ff', '#bb66ff', '#ff3366', '#00ff88'], gradient: 'linear-gradient(160deg, #050510, #0a0a1e, #0f0828)' },
  { id: 'ghibli', name: '🌿 Ghibli', anime: true, colors: ['#0a1a10', '#a5d6a7', '#81d4fa', '#fff9c4', '#e8f5e9'], gradient: 'linear-gradient(150deg, #0a1a10, #142518, #1a3020)' },
];

const ANIME_IDS = new Set(THEMES.filter(t => t.anime).map(t => t.id));

/** Apply background image + opacity to the DOM */
function applyBgImage(themeId: string) {
  let el = document.getElementById('sb-theme-bg');
  const dataUrl = localStorage.getItem(`sb_theme_bg_${themeId}`) || '';
  const opacity = parseFloat(localStorage.getItem(`sb_theme_bg_opacity_${themeId}`) || '0.3');

  if (!dataUrl || !ANIME_IDS.has(themeId)) {
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
  el.style.backgroundImage = `url(${dataUrl})`;
  el.style.opacity = String(opacity);
}

export function ThemePicker() {
  const [current, setCurrent] = useState(() => localStorage.getItem('sb_theme') || 'mocha');
  const [bgPreview, setBgPreview] = useState<string>('');
  const [bgOpacity, setBgOpacity] = useState(0.3);
  const fileRef = useRef<HTMLInputElement>(null);

  const isAnime = ANIME_IDS.has(current);

  // Sync state on mount & theme-change
  useEffect(() => {
    const sync = () => {
      const id = localStorage.getItem('sb_theme') || 'mocha';
      setCurrent(id);
      setBgPreview(localStorage.getItem(`sb_theme_bg_${id}`) || '');
      setBgOpacity(parseFloat(localStorage.getItem(`sb_theme_bg_opacity_${id}`) || '0.3'));
      applyBgImage(id);
    };
    sync();
    window.addEventListener('theme-change', sync);
    return () => window.removeEventListener('theme-change', sync);
  }, []);

  const handleSelect = (id: string) => {
    document.documentElement.setAttribute('data-theme', id);
    localStorage.setItem('sb_theme', id);
    setCurrent(id);
    setBgPreview(localStorage.getItem(`sb_theme_bg_${id}`) || '');
    setBgOpacity(parseFloat(localStorage.getItem(`sb_theme_bg_opacity_${id}`) || '0.3'));
    applyBgImage(id);
    window.dispatchEvent(new Event('theme-change'));
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      const dataUrl = reader.result as string;
      localStorage.setItem(`sb_theme_bg_${current}`, dataUrl);
      setBgPreview(dataUrl);
      applyBgImage(current);
    };
    reader.readAsDataURL(file);
    e.target.value = '';
  };

  const handleRemoveBg = () => {
    localStorage.removeItem(`sb_theme_bg_${current}`);
    setBgPreview('');
    applyBgImage(current);
  };

  const handleOpacityChange = (val: number) => {
    setBgOpacity(val);
    localStorage.setItem(`sb_theme_bg_opacity_${current}`, String(val));
    applyBgImage(current);
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
          <input ref={fileRef} type="file" accept="image/*" style={{ display: 'none' }} onChange={handleFileChange} />
          <div className="theme-bg-controls">
            <button className="theme-bg-btn choose" onClick={() => fileRef.current?.click()}>
              {t.settings.chooseBg}
            </button>
            {bgPreview && (
              <button className="theme-bg-btn remove" onClick={handleRemoveBg}>
                {t.settings.removeBg}
              </button>
            )}
          </div>
          {bgPreview && (
            <>
              <div className="theme-bg-preview-wrap">
                <img src={bgPreview} alt="bg" className="theme-bg-preview" />
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
