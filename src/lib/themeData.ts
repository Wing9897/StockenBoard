import { getTransport } from './transport';
import { STORAGE_KEYS } from './storageKeys';

type ThemeEntry = {
  id: string;
  name: string;
  colors: string[];
  anime?: boolean;
  gradient?: string;
};

export const THEMES: ThemeEntry[] = [
  // Dark themes
  { id: 'mocha', name: 'Mocha', colors: ['#282840', '#3c3e58', '#89b4fa', '#a6e3a1', '#cdd6f4'] },
  { id: 'midnight', name: 'Midnight', colors: ['#1e1e22', '#2a2a30', '#6ea8fe', '#75d47a', '#e0e0e0'] },
  { id: 'nord', name: 'Nord', colors: ['#2e3440', '#3b4252', '#88c0d0', '#a3be8c', '#eceff4'] },
  { id: 'gruvbox', name: 'Gruvbox', colors: ['#282828', '#3c3836', '#fabd2f', '#b8bb26', '#ebdbb2'] },
  { id: 'solarized', name: 'Solarized', colors: ['#003845', '#0e4250', '#268bd2', '#859900', '#93a1a1'] },
  // Light themes
  { id: 'snow', name: '❄️ Snow', colors: ['#f0f2f5', '#dde0e6', '#3b82f6', '#10b981', '#1f2937'] },
  { id: 'ocean-light', name: '🌊 Ocean', anime: true, colors: ['#e4f0f8', '#bdd8ea', '#0284c7', '#0d9488', '#0c4a6e'], gradient: 'linear-gradient(145deg, #d8eaf5, #e4f0f8, #dceef8)' },
  { id: 'matcha', name: '🍵 Matcha', anime: true, colors: ['#e8f0e4', '#c8d6be', '#4d7c4d', '#8fbc8f', '#2d4a2d'], gradient: 'linear-gradient(140deg, #e8f0e4, #e0e8dc, #e4ede0)' },
  { id: 'sakuralight', name: '🌸 Sakura Light', anime: true, colors: ['#f2e6eb', '#dcbeca', '#d63384', '#f48fb1', '#4a1830'], gradient: 'linear-gradient(145deg, #f2e6eb, #eee0e6, #f0e4e8)' },
  // Anime/special themes (dark with background support)
  { id: 'sakura', name: '🌸 Sakura', anime: true, colors: ['#2d1423', '#f48fb1', '#ce93d8', '#fce4ec', '#90caf9'], gradient: 'linear-gradient(135deg, #2d1423, #3d1a30, #2a1028)' },
  { id: 'cyberpunk', name: '⚡ Cyberpunk', anime: true, colors: ['#050510', '#00d4ff', '#bb66ff', '#ff3366', '#00ff88'], gradient: 'linear-gradient(160deg, #050510, #0a0a1e, #0f0828)' },
  // Pattern themes
  { id: 'blueprint', name: '📐 Blueprint', anime: true, colors: ['#0a1628', '#122040', '#4fc3f7', '#66bb6a', '#d0dff0'], gradient: 'linear-gradient(160deg, #0a1628, #081220, #0a1628)' },
  { id: 'botanical', name: '🌿 Botanical', anime: true, colors: ['#f0efe8', '#d6d4ca', '#4a7c59', '#5a8a4a', '#2b2a1e'], gradient: 'linear-gradient(140deg, #f0efe8, #e8e7df, #f0efe8)' },
  { id: 'geometric', name: '🔷 Geometric', anime: true, colors: ['#1a1a2e', '#252542', '#00adb5', '#6bcc7a', '#d4d4f0'], gradient: 'linear-gradient(150deg, #1a1a2e, #141426, #1a1a2e)' },
  { id: 'wave', name: '🌊 Wave', anime: true, colors: ['#0a192f', '#122a4a', '#64ffda', '#69f0ae', '#ccd6e8'], gradient: 'linear-gradient(155deg, #0a192f, #071220, #0a192f)' },
  { id: 'aurora', name: '✨ Aurora', anime: true, colors: ['#0f0f1a', '#1a1a30', '#00d4aa', '#8b5cf6', '#d8d8f0'], gradient: 'linear-gradient(145deg, #0f0f1a, #0a0a14, #0f0f1a)' },
];

export const ANIME_IDS = new Set(THEMES.filter(t => t.anime).map(t => t.id));

/** Apply background image + opacity to the DOM */
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

/** 從 Rust 載入 bg 並套用到 DOM，回傳 data URL 或 null */
export async function loadBgForTheme(themeId: string): Promise<string | null> {
  try {
    const filePath = await getTransport().invoke<string | null>('get_theme_bg_path', { themeId });
    if (filePath) {
      const dataUrl = await getTransport().invoke<string>('read_local_file_base64', { path: filePath });
      const opacity = parseFloat(localStorage.getItem(STORAGE_KEYS.themeBgOpacity(themeId)) || '0.3');
      applyBgImage(dataUrl, opacity);
      return dataUrl;
    }
  } catch { /* ignore */ }
  applyBgImage(null, 0);
  return null;
}
