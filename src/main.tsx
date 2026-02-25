import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./theme.css";

// Apply saved theme before render
const savedTheme = localStorage.getItem('sb_theme') || 'mocha';
document.documentElement.setAttribute('data-theme', savedTheme);

// Apply saved background image for anime themes
const ANIME_THEMES = ['sakura', 'cyberpunk', 'ghibli'];
if (ANIME_THEMES.includes(savedTheme)) {
  const bgUrl = localStorage.getItem(`sb_theme_bg_${savedTheme}`);
  if (bgUrl) {
    const opacity = localStorage.getItem(`sb_theme_bg_opacity_${savedTheme}`) || '0.3';
    const el = document.createElement('div');
    el.id = 'sb-theme-bg';
    Object.assign(el.style, {
      position: 'fixed', inset: '0', zIndex: '0', pointerEvents: 'none',
      backgroundSize: 'cover', backgroundPosition: 'center', backgroundRepeat: 'no-repeat',
      backgroundImage: `url(${bgUrl})`, opacity,
    });
    document.body.prepend(el);
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
