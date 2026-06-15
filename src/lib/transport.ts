/**
 * Transport abstraction layer — auto-detects runtime environment
 * and routes backend calls through Tauri IPC or HTTP fetch accordingly.
 *
 * Usage:
 *   const transport = createTransport();
 *   const subs = await transport.invoke<Subscription[]>('list_all_subscriptions');
 *   const unlisten = transport.listen('price-update', (payload) => { ... });
 */

import { HttpTransport } from './transportWs';

// Re-export mapCommandToHttp so existing consumers that import from './transport' still work.
export { mapCommandToHttp } from './transportRoutes';

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

