/**
 * Transport abstraction layer — auto-detects runtime environment
 * and routes backend calls through Tauri IPC or HTTP fetch accordingly.
 *
 * Usage:
 *   const transport = createTransport();
 *   const subs = await transport.invoke<Subscription[]>('list_all_subscriptions');
 *   const unlisten = transport.listen('price-update', (payload) => { ... });
 */

/**
 * Unified transport interface for communicating with the backend.
 * Both Tauri IPC and HTTP REST share this contract, allowing the React SPA
 * to work identically in desktop and browser environments.
 */
export interface Transport {
  /**
   * Invoke a backend command by name, optionally passing arguments.
   * Resolves with the typed response from the backend.
   */
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;

  /**
   * Subscribe to a real-time event stream by event name.
   * Returns an unsubscribe function that stops listening when called.
   */
  listen(event: string, handler: (payload: unknown) => void): () => void;
}

/**
 * Detect whether the application is running inside a Tauri webview.
 * Tauri 2 injects `window.__TAURI_INTERNALS__` by default.
 * `window.__TAURI__` is only available when `app.withGlobalTauri` is true.
 */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * TauriTransport — routes calls through Tauri IPC invoke and event listener.
 * Uses dynamic imports to avoid bundling Tauri deps when running in browser mode.
 */
class TauriTransport implements Transport {
  async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(command, args);
  }

  listen(event: string, handler: (payload: unknown) => void): () => void {
    let unlistenFn: (() => void) | null = null;
    let cancelled = false;

    // Start listening asynchronously; store the unlisten function when resolved.
    import('@tauri-apps/api/event').then(({ listen }) => {
      if (cancelled) return;
      listen<unknown>(event, (ev) => {
        handler(ev.payload);
      }).then((unlisten) => {
        if (cancelled) {
          // Caller already unsubscribed before the listener was established
          unlisten();
        } else {
          unlistenFn = unlisten;
        }
      });
    });

    // Return a synchronous unsubscribe function
    return () => {
      cancelled = true;
      if (unlistenFn) {
        unlistenFn();
        unlistenFn = null;
      }
    };
  }
}

/**
 * Maps a Tauri IPC command name and its arguments to the corresponding
 * HTTP method, URL path, and optional JSON body for the REST API.
 *
 * Exported for unit testing (task 8.7).
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

/**
 * HttpTransport — routes calls through HTTP fetch to the server REST API
 * and uses WebSocket for real-time events.
 */
class HttpTransport implements Transport {
  private ws: WebSocket | null = null;
  private eventHandlers = new Map<string, Set<(payload: unknown) => void>>();
  private reconnectAttempt = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private intentionallyClosed = false;

  async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    const { method, path, body } = mapCommandToHttp(command, args);

    const headers: Record<string, string> = {};
    if (body) {
      headers['Content-Type'] = 'application/json';
    }

    const response = await fetch(`/api${path}`, {
      method,
      headers,
      body: body ?? undefined,
    });

    // Handle 204 No Content (typical for DELETE operations)
    if (response.status === 204) {
      return undefined as unknown as T;
    }

    // Non-OK responses that aren't 204 — throw with the response text
    if (!response.ok) {
      const text = await response.text();
      let message = text;
      try {
        const parsed = JSON.parse(text);
        if (parsed?.error?.message) {
          message = parsed.error.message;
        }
      } catch {
        // Use raw text as-is
      }
      throw new Error(message);
    }

    // Unwrap the response envelope: { data: T } or { error: { code, message } }
    const envelope = await response.json();

    if (envelope.error) {
      throw new Error(envelope.error.message || envelope.error.code || 'Unknown error');
    }

    return envelope.data as T;
  }

  listen(event: string, handler: (payload: unknown) => void): () => void {
    this.ensureWebSocket();

    // Register handler for this event type
    let handlers = this.eventHandlers.get(event);
    if (!handlers) {
      handlers = new Set();
      this.eventHandlers.set(event, handlers);
    }
    handlers.add(handler);

    // Return an unsubscribe function that removes this specific handler
    return () => {
      const set = this.eventHandlers.get(event);
      if (set) {
        set.delete(handler);
        if (set.size === 0) {
          this.eventHandlers.delete(event);
        }
      }
    };
  }

  /**
   * Lazily establish a WebSocket connection to /api/ws.
   * Handles reconnection with exponential backoff on connection loss.
   */
  private ensureWebSocket(): void {
    if (this.ws && (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING)) {
      return;
    }

    this.intentionallyClosed = false;
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    this.ws = new WebSocket(`${protocol}//${location.host}/api/ws`);

    this.ws.onopen = () => {
      // Reset reconnect counter on successful connection
      this.reconnectAttempt = 0;
    };

    this.ws.onmessage = (event: MessageEvent) => {
      try {
        const message = JSON.parse(event.data) as { type: string; data: unknown; timestamp: number };
        const handlers = this.eventHandlers.get(message.type);
        if (handlers) {
          for (const handler of handlers) {
            handler(message.data);
          }
        }
      } catch {
        // Ignore messages that aren't valid JSON or don't match the envelope format
      }
    };

    this.ws.onclose = () => {
      this.ws = null;
      if (!this.intentionallyClosed && this.eventHandlers.size > 0) {
        this.scheduleReconnect();
      }
    };

    this.ws.onerror = () => {
      // The onclose handler will fire after onerror, triggering reconnection
    };
  }

  /**
   * Schedule a reconnection attempt using exponential backoff.
   * Delays: 1s, 2s, 4s, 8s, 16s, 30s (capped).
   */
  private scheduleReconnect(): void {
    if (this.reconnectTimer) {
      return;
    }

    const baseDelay = 1000; // 1 second
    const maxDelay = 30000; // 30 seconds
    const delay = Math.min(baseDelay * Math.pow(2, this.reconnectAttempt), maxDelay);
    this.reconnectAttempt++;

    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      // Only reconnect if there are still active handlers
      if (this.eventHandlers.size > 0) {
        this.ensureWebSocket();
      }
    }, delay);
  }
}

/**
 * Factory that creates the appropriate transport based on the runtime environment.
 * - In a Tauri webview → TauriTransport (IPC)
 * - In a standard browser → HttpTransport (HTTP + WebSocket)
 */
export function createTransport(): Transport {
  if (isTauri()) {
    return new TauriTransport();
  }
  return new HttpTransport();
}

/**
 * Module-level lazy singleton transport instance.
 * Deferred creation ensures `window.__TAURI__` is available when first accessed
 * (Tauri injects it before user scripts run, but module-level code may race in dev).
 */
let _transport: Transport | null = null;
export function getTransport(): Transport {
  if (!_transport) {
    _transport = createTransport();
  }
  return _transport;
}

/** @deprecated Use getTransport() for lazy initialization. Kept for backward compat. */
export const transport = new Proxy({} as Transport, {
  get(_target, prop) {
    return (getTransport() as unknown as Record<string | symbol, unknown>)[prop];
  },
});
