import React from "react";
import ReactDOM from "react-dom/client";
import { TransportProvider } from './lib/transportProvider';
import { ANIME_IDS, loadBgForTheme } from './lib/themeData';
import { STORAGE_KEYS } from './lib/storageKeys';
import App from "./App";
import "./theme.css";

// Apply saved theme before render (synchronous — theme CSS only)
const savedTheme = localStorage.getItem(STORAGE_KEYS.THEME) || 'mocha';
document.documentElement.setAttribute('data-theme', savedTheme);

// Apply background image for anime themes (async — reuse ThemePicker logic)
if (ANIME_IDS.has(savedTheme)) {
  loadBgForTheme(savedTheme).catch(() => { });
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <TransportProvider>
      <App />
    </TransportProvider>
  </React.StrictMode>,
);
