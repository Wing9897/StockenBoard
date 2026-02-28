import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { t } from '../../lib/i18n';
import { THEMES, ANIME_IDS, applyBgImage, loadBgForTheme } from '../../lib/themeData';
import { STORAGE_KEYS } from '../../lib/storageKeys';
import './Settings.css';

export function ThemePicker() {
  const [current, setCurrent] = useState(() => localStorage.getItem(STORAGE_KEYS.THEME) || 'mocha');
  const [bgUrl, setBgUrl] = useState<string | null>(null);
  const [bgOpacity, setBgOpacity] = useState(0.3);

  const isAnime = ANIME_IDS.has(current);

  useEffect(() => {
    let cancelled = false;
    const handler = async () => {
      const url = await loadBgForTheme(localStorage.getItem(STORAGE_KEYS.THEME) || 'mocha');
      if (!cancelled) {
        setCurrent(localStorage.getItem(STORAGE_KEYS.THEME) || 'mocha');
        setBgOpacity(parseFloat(localStorage.getItem(STORAGE_KEYS.themeBgOpacity(localStorage.getItem(STORAGE_KEYS.THEME) || 'mocha')) || '0.3'));
        setBgUrl(url);
      }
    };
    handler();
    window.addEventListener('theme-change', handler);
    return () => { cancelled = true; window.removeEventListener('theme-change', handler); };
  }, []);

  const handleSelect = async (id: string) => {
    document.documentElement.setAttribute('data-theme', id);
    localStorage.setItem(STORAGE_KEYS.THEME, id);
    setCurrent(id);
    setBgOpacity(parseFloat(localStorage.getItem(STORAGE_KEYS.themeBgOpacity(id)) || '0.3'));
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
    localStorage.setItem(STORAGE_KEYS.themeBgOpacity(current), String(val));
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
            <button className="theme-bg-btn choose" onClick={handleChooseBg} aria-label={t.settings.chooseBg}>{t.settings.chooseBg}</button>
            {bgUrl && (
              <button className="theme-bg-btn remove" onClick={handleRemoveBg} aria-label={t.settings.removeBg}>{t.settings.removeBg}</button>
            )}
          </div>
          {bgUrl && (
            <>
              <div className="theme-bg-preview-wrap">
                <img src={bgUrl} alt="bg" className="theme-bg-preview" />
              </div>
              <div className="theme-bg-opacity">
                <label>{t.settings.bgOpacity}</label>
                <input type="range" min="0.05" max="0.6" step="0.05" value={bgOpacity} onChange={e => handleOpacityChange(parseFloat(e.target.value))} />
                <span>{Math.round(bgOpacity * 100)}%</span>
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}
