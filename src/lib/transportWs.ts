/**
 * HttpTransport — routes calls through HTTP fetch to the server REST API
 * and uses WebSocket for real-time events with automatic reconnection.
 */

import { mapCommandToHttp } from './transportRoutes';
import type { Transport } from './transport';

/**
 * HttpTransport — routes calls through HTTP fetch to the server REST API
 * and uses WebSocket for real-time events.
 */
export class HttpTransport implements Transport {
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
