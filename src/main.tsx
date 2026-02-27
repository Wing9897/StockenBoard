import React from "react";
import ReactDOM from "react-dom/client";
import { invoke } from '@tauri-apps/api/core';
import { ANIME_IDS, loadBgForTheme } from './lib/themeData';
import App from "./App";
import "./theme.css";

// Apply saved theme before render (synchronous — theme CSS only)
const savedTheme = localStorage.getItem('sb_theme') || 'mocha';
document.documentElement.setAttribute('data-theme', savedTheme);

// Apply background image for anime themes (async — reuse ThemePicker logic)
if (ANIME_IDS.has(savedTheme)) {
  loadBgForTheme(savedTheme).catch(() => {});
}

// Restore unattended polling state from localStorage to Rust backend
if (localStorage.getItem('sb_unattended') === '1') {
  invoke('set_unattended_polling', { enabled: true }).catch(() => {});
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
