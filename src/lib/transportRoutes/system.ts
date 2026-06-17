/**
 * System, icons, data, recording, file ops, theme, polling visibility, and API settings route mappings.
 */

import type { RouteMapper } from './subscriptions';

export const systemRoutes: Record<string, RouteMapper> = {
  // --- Data ---
  export_data: () => ({ method: 'GET', path: '/data/export' }),
  import_data: (a) => ({
    method: 'POST',
    path: '/data/import',
    body: JSON.stringify(a.data ?? a),
  }),

  // --- System ---
  get_api_port: () => ({ method: 'GET', path: '/system/config', extractField: 'api_port' }),
  set_api_port: (a) => ({
    method: 'PUT',
    path: '/system/config',
    body: JSON.stringify({ api_port: a.port }),
  }),
  get_unattended_polling: () => ({ method: 'GET', path: '/system/config', extractField: 'unattended_polling' }),
  set_unattended_polling: (a) => ({
    method: 'PUT',
    path: '/system/config',
    body: JSON.stringify({ unattended_polling: a.enabled }),
  }),
  reload_polling: () => ({
    method: 'POST',
    path: '/system/reload-polling',
  }),
  reset_all_data: () => ({
    method: 'POST',
    path: '/system/reset',
  }),
  get_data_dir: () => ({ method: 'GET', path: '/system/data-dir' }),

  // --- Icons ---
  set_icon: (a) => ({
    method: 'POST',
    path: `/icons/${encodeURIComponent(String(a.symbol))}`,
    body: JSON.stringify(a),
  }),
  remove_icon: (a) => ({
    method: 'DELETE',
    path: `/icons/${encodeURIComponent(String(a.symbol))}`,
  }),
  get_icons_dir: () => ({ method: 'GET', path: '/icons/dir' }),
  download_logos: (a) => ({
    method: 'POST',
    path: '/icons/download-logos',
    body: JSON.stringify(a),
  }),
  clear_all_icons: () => ({ method: 'DELETE', path: '/icons/clear-all', extractField: 'deleted' }),
  download_single_icon: (a) => ({
    method: 'POST',
    path: `/icons/${encodeURIComponent(String(a.saveAs ?? a.save_as))}/download`,
    body: JSON.stringify({ symbol: a.symbol }),
  }),
  search_icons: (a) => ({
    method: 'GET',
    path: `/icons/search?symbol=${encodeURIComponent(String(a.symbol))}`,
  }),
  save_icon_from_data: (a) => ({
    method: 'POST',
    path: `/icons/${encodeURIComponent(String(a.saveAs ?? a.save_as))}/save`,
    body: JSON.stringify({ data_url: a.dataUrl ?? a.data_url }),
  }),

  // --- Polling Visibility ---
  set_visible_subscriptions: (a) => ({
    method: 'PUT',
    path: '/system/visible-subscriptions',
    body: JSON.stringify(a),
  }),

  // --- Theme Background ---
  save_theme_bg: () => ({
    method: 'POST',
    path: '/system/desktop-only',
    body: JSON.stringify({ command: 'save_theme_bg', error: 'File dialog not available in web mode' }),
  }),
  remove_theme_bg: (a) => ({
    method: 'DELETE',
    path: `/system/theme-bg/${encodeURIComponent(String(a.theme_id))}`,
  }),
  get_theme_bg_path: (a) => ({
    method: 'GET',
    path: `/system/theme-bg/${encodeURIComponent(String(a.theme_id))}`,
  }),

  // --- File Operations (Desktop Only) ---
  export_file: () => ({
    method: 'POST',
    path: '/system/desktop-only',
    body: JSON.stringify({ command: 'export_file', error: 'File dialog not available in web mode' }),
  }),
  import_file: () => ({
    method: 'POST',
    path: '/system/desktop-only',
    body: JSON.stringify({ command: 'import_file', error: 'File dialog not available in web mode' }),
  }),
  read_local_file_base64: (a) => ({
    method: 'GET',
    path: `/system/read-file?path=${encodeURIComponent(String(a.path))}`,
  }),

  // --- API Settings ---
  get_api_enabled: () => ({ method: 'GET', path: '/system/config', extractField: 'api_enabled' }),
  set_api_enabled: (a) => ({
    method: 'PUT',
    path: '/system/config',
    body: JSON.stringify({ api_enabled: a.enabled }),
  }),

  // --- WebSocket Stream Control ---
  start_ws_stream: (a) => ({
    method: 'POST',
    path: '/ws/stream/start',
    body: JSON.stringify({ provider_id: a.providerId, symbols: a.symbols }),
  }),
  stop_ws_stream: (a) => ({
    method: 'POST',
    path: '/ws/stream/stop',
    body: JSON.stringify({ provider_id: a.providerId }),
  }),
};
