/**
 * HTTP route mapping for Tauri IPC command names.
 *
 * Maps each backend command to its corresponding REST endpoint
 * (method, path, optional JSON body).
 */

/**
 * Maps a Tauri IPC command name and its arguments to the corresponding
 * HTTP method, URL path, and optional JSON body for the REST API.
 *
 * Exported for unit testing and used by HttpTransport.
 */
export function mapCommandToHttp(
  command: string,
  args?: Record<string, unknown>
): { method: string; path: string; body?: string } {
  const a = args ?? {};

  const routes: Record<
    string,
    (a: Record<string, unknown>) => { method: string; path: string; body?: string }
  > = {
    // --- Subscriptions ---
    list_all_subscriptions: () => ({ method: 'GET', path: '/subscriptions' }),
    list_subscriptions: (a) => ({
      method: 'GET',
      path: `/subscriptions?type=${encodeURIComponent(String(a.type ?? ''))}`,
    }),
    add_subscription: (a) => ({
      method: 'POST',
      path: '/subscriptions',
      body: JSON.stringify(a),
    }),
    add_subscriptions_batch: (a) => ({
      method: 'POST',
      path: '/subscriptions/batch',
      body: JSON.stringify(a),
    }),
    update_subscription: (a) => ({
      method: 'PUT',
      path: `/subscriptions/${encodeURIComponent(String(a.id))}`,
      body: JSON.stringify(a),
    }),
    remove_subscription: (a) => ({
      method: 'DELETE',
      path: `/subscriptions/${encodeURIComponent(String(a.id))}`,
    }),
    remove_subscriptions: (a) => ({
      method: 'DELETE',
      path: '/subscriptions/batch',
      body: JSON.stringify(a),
    }),

    // --- Views ---
    list_views: (a) => ({
      method: 'GET',
      path: `/views?type=${encodeURIComponent(String(a.type ?? ''))}`,
    }),
    create_view: (a) => ({
      method: 'POST',
      path: '/views',
      body: JSON.stringify(a),
    }),
    rename_view: (a) => ({
      method: 'PUT',
      path: `/views/${encodeURIComponent(String(a.id))}`,
      body: JSON.stringify(a),
    }),
    delete_view: (a) => ({
      method: 'DELETE',
      path: `/views/${encodeURIComponent(String(a.id))}`,
    }),
    add_sub_to_view: (a) => ({
      method: 'POST',
      path: `/views/${encodeURIComponent(String(a.id))}/subscriptions`,
      body: JSON.stringify(a),
    }),
    remove_sub_from_view: (a) => ({
      method: 'DELETE',
      path: `/views/${encodeURIComponent(String(a.view_id))}/subscriptions/${encodeURIComponent(String(a.sub_id))}`,
    }),
    get_view_sub_counts: () => ({ method: 'GET', path: '/views/sub-counts' }),
    get_view_subscription_ids: (a) => ({
      method: 'GET',
      path: `/views/${encodeURIComponent(String(a.viewId))}/subscription-ids`,
    }),

    // --- Providers ---
    get_all_providers: () => ({ method: 'GET', path: '/providers' }),
    enable_provider: (a) => ({
      method: 'POST',
      path: `/providers/${encodeURIComponent(String(a.id))}/enable`,
      body: JSON.stringify(a),
    }),

    // --- Provider Settings ---
    list_provider_settings: () => ({ method: 'GET', path: '/provider-settings' }),
    upsert_provider_settings: (a) => ({
      method: 'PUT',
      path: `/provider-settings/${encodeURIComponent(String(a.id))}`,
      body: JSON.stringify(a),
    }),
    has_api_key: (a) => ({
      method: 'GET',
      path: `/provider-settings/${encodeURIComponent(String(a.id))}/has-key`,
    }),

    // --- Notifications: Rules ---
    create_notification_rule: (a) => ({
      method: 'POST',
      path: '/notifications/rules',
      body: JSON.stringify(a),
    }),
    list_notification_rules: () => ({ method: 'GET', path: '/notifications/rules' }),
    update_notification_rule: (a) => ({
      method: 'PUT',
      path: `/notifications/rules/${encodeURIComponent(String(a.id))}`,
      body: JSON.stringify(a),
    }),
    delete_notification_rule: (a) => ({
      method: 'DELETE',
      path: `/notifications/rules/${encodeURIComponent(String(a.id))}`,
    }),
    toggle_notification_rule: (a) => ({
      method: 'POST',
      path: `/notifications/rules/${encodeURIComponent(String(a.id))}/toggle`,
      body: JSON.stringify(a),
    }),

    // --- Notifications: Channels ---
    save_notification_channel: (a) => ({
      method: 'POST',
      path: '/notifications/channels',
      body: JSON.stringify(a),
    }),
    list_notification_channels: () => ({ method: 'GET', path: '/notifications/channels' }),
    delete_notification_channel: (a) => ({
      method: 'DELETE',
      path: `/notifications/channels/${encodeURIComponent(String(a.id))}`,
    }),
    test_notification_channel: (a) => ({
      method: 'POST',
      path: `/notifications/channels/${encodeURIComponent(String(a.id))}/test`,
      body: JSON.stringify(a),
    }),

    // --- Notifications: History & Cooldown ---
    get_notification_history: () => ({ method: 'GET', path: '/notifications/history' }),
    get_notification_global_cooldown: () => ({ method: 'GET', path: '/notifications/cooldown' }),
    set_notification_global_cooldown: (a) => ({
      method: 'PUT',
      path: '/notifications/cooldown',
      body: JSON.stringify(a),
    }),

    // --- AI ---
    save_ai_provider_config: (a) => ({
      method: 'POST',
      path: '/ai/config',
      body: JSON.stringify(a),
    }),
    get_ai_provider_config: () => ({ method: 'GET', path: '/ai/config' }),
    test_ai_connection: (a) => ({
      method: 'POST',
      path: '/ai/test',
      body: JSON.stringify(a),
    }),
    list_ai_models: () => ({ method: 'GET', path: '/ai/models' }),

    // --- Prices ---
    fetch_asset_price: (a) => ({
      method: 'GET',
      path: `/prices/fetch/${encodeURIComponent(String(a.provider))}/${encodeURIComponent(String(a.symbol))}`,
    }),
    fetch_multiple_prices: (a) => ({
      method: 'POST',
      path: '/prices/fetch-multiple',
      body: JSON.stringify(a),
    }),
    get_cached_prices: () => ({ method: 'GET', path: '/prices/cached' }),
    get_poll_ticks: () => ({ method: 'GET', path: '/prices/poll-ticks' }),

    // --- History ---
    get_price_history: (a) => ({
      method: 'GET',
      path: `/history/${encodeURIComponent(String(a.sub_id))}`,
    }),
    get_history_stats: () => ({ method: 'GET', path: '/history/stats' }),
    cleanup_history: (a) => ({
      method: 'POST',
      path: '/history/cleanup',
      body: JSON.stringify(a),
    }),
    purge_all_history: () => ({ method: 'DELETE', path: '/history' }),
    delete_subscription_history: (a) => ({
      method: 'DELETE',
      path: `/history/${encodeURIComponent(String(a.sub_id))}`,
    }),

    // --- Data ---
    export_data: () => ({ method: 'GET', path: '/data/export' }),
    import_data: (a) => ({
      method: 'POST',
      path: '/data/import',
      body: JSON.stringify(a),
    }),

    // --- System ---
    get_api_port: () => ({ method: 'GET', path: '/system/config' }),
    set_api_port: (a) => ({
      method: 'PUT',
      path: '/system/config',
      body: JSON.stringify(a),
    }),
    get_unattended_polling: () => ({ method: 'GET', path: '/system/config' }),
    set_unattended_polling: (a) => ({
      method: 'PUT',
      path: '/system/config',
      body: JSON.stringify(a),
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
    get_icons_dir: () => ({ method: 'GET', path: '/icons' }),
    download_logos: (a) => ({
      method: 'POST',
      path: '/icons/download-logos',
      body: JSON.stringify(a),
    }),

    // --- DEX ---
    lookup_dex_pool: (a) => ({
      method: 'GET',
      path: `/dex/pool/${encodeURIComponent(String(a.provider))}/${encodeURIComponent(String(a.address))}`,
    }),
  };

  const mapper = routes[command];
  if (!mapper) {
    // Fallback for unknown commands — best-effort GET mapping
    return { method: 'GET', path: `/${command}` };
  }
  return mapper(a);
}
