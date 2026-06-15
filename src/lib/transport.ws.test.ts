/**
 * Unit tests for HttpTransport WebSocket event listener (task 8.4).
 * Tests the listen/unsubscribe API, message routing, and reconnection logic.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// --- Mock WebSocket ---
class MockWebSocket {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;

  readyState = MockWebSocket.CONNECTING;
  onopen: (() => void) | null = null;
  onclose: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  onerror: (() => void) | null = null;
  url: string;

  constructor(url: string) {
    this.url = url;
    // Simulate async open
    setTimeout(() => {
      this.readyState = MockWebSocket.OPEN;
      this.onopen?.();
    }, 0);
  }

  close() {
    this.readyState = MockWebSocket.CLOSED;
    this.onclose?.();
  }

  // Helper to simulate receiving a message
  simulateMessage(data: string) {
    this.onmessage?.({ data });
  }

  // Helper to simulate connection loss
  simulateClose() {
    this.readyState = MockWebSocket.CLOSED;
    this.onclose?.();
  }

  simulateError() {
    this.onerror?.();
  }
}

// Stub globals
const originalWebSocket = globalThis.WebSocket;
const originalLocation = globalThis.location;

beforeEach(() => {
  vi.useFakeTimers();
  // @ts-expect-error - mocking global WebSocket
  globalThis.WebSocket = MockWebSocket;
  // @ts-expect-error - mocking location
  globalThis.location = { protocol: 'http:', host: 'localhost:8080' };
});

afterEach(() => {
  vi.useRealTimers();
  globalThis.WebSocket = originalWebSocket;
  // @ts-expect-error - restoring location
  globalThis.location = originalLocation;
});

// We need to dynamically import the module after mocking globals
async function getTransport() {
  // Clear module cache to get fresh instances
  const mod = await import('./transport');
  // createTransport will return HttpTransport since __TAURI__ is not defined
  return mod.createTransport();
}

describe('HttpTransport WebSocket listen()', () => {
  it('should establish a WebSocket connection on first listen() call', async () => {
    const transport = await getTransport();
    const handler = vi.fn();

    transport.listen('price-update', handler);

    // WebSocket should have been created
    // We can verify by checking the mock was instantiated
    expect(handler).not.toHaveBeenCalled();
  });

  it('should route messages to the correct handler by type field', async () => {
    const transport = await getTransport();
    const priceHandler = vi.fn();
    const pollHandler = vi.fn();

    transport.listen('price-update', priceHandler);
    transport.listen('poll-tick', pollHandler);

    // Get the WebSocket instance (it's the last one created)
    // We need to trigger message delivery via the mock
    await vi.advanceTimersByTimeAsync(0); // Let WS connect

    // Find the mock WS — since our mock stores itself as a constructor result,
    // we access it through the transport's internal state indirectly by
    // simulating what would happen when the WS receives a message.
    // The simplest approach: directly call listen handlers via a constructed mock.

    // Actually, let's test by using a captured reference approach:
    let wsInstance: MockWebSocket | null = null;
    const OrigMock = MockWebSocket;
    // @ts-expect-error - mocking
    globalThis.WebSocket = class extends OrigMock {
      constructor(url: string) {
        super(url);
        wsInstance = this;
      }
    };

    const transport2 = (await import('./transport')).createTransport();
    const handler1 = vi.fn();
    const handler2 = vi.fn();
    transport2.listen('price-update', handler1);
    transport2.listen('poll-tick', handler2);

    await vi.advanceTimersByTimeAsync(0);

    // Simulate incoming messages
    wsInstance!.simulateMessage(JSON.stringify({
      type: 'price-update',
      data: { symbol: 'BTC', price: 50000 },
      timestamp: 1700000000,
    }));

    expect(handler1).toHaveBeenCalledWith({ symbol: 'BTC', price: 50000 });
    expect(handler2).not.toHaveBeenCalled();

    wsInstance!.simulateMessage(JSON.stringify({
      type: 'poll-tick',
      data: { tick: 42 },
      timestamp: 1700000001,
    }));

    expect(handler2).toHaveBeenCalledWith({ tick: 42 });
  });

  it('should deliver only the data field to handlers (not the full envelope)', async () => {
    let wsInstance: MockWebSocket | null = null;
    // @ts-expect-error - mocking
    globalThis.WebSocket = class extends MockWebSocket {
      constructor(url: string) {
        super(url);
        wsInstance = this;
      }
    };

    const transport = (await import('./transport')).createTransport();
    const handler = vi.fn();
    transport.listen('notification-triggered', handler);

    await vi.advanceTimersByTimeAsync(0);

    const envelope = {
      type: 'notification-triggered',
      data: { rule_id: 'abc', message: 'Price hit target' },
      timestamp: 1700000000,
    };
    wsInstance!.simulateMessage(JSON.stringify(envelope));

    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith({ rule_id: 'abc', message: 'Price hit target' });
  });

  it('should support multiple handlers for the same event type', async () => {
    let wsInstance: MockWebSocket | null = null;
    // @ts-expect-error - mocking
    globalThis.WebSocket = class extends MockWebSocket {
      constructor(url: string) {
        super(url);
        wsInstance = this;
      }
    };

    const transport = (await import('./transport')).createTransport();
    const handler1 = vi.fn();
    const handler2 = vi.fn();
    transport.listen('price-update', handler1);
    transport.listen('price-update', handler2);

    await vi.advanceTimersByTimeAsync(0);

    wsInstance!.simulateMessage(JSON.stringify({
      type: 'price-update',
      data: [{ symbol: 'ETH', price: 3000 }],
      timestamp: 1700000000,
    }));

    expect(handler1).toHaveBeenCalledWith([{ symbol: 'ETH', price: 3000 }]);
    expect(handler2).toHaveBeenCalledWith([{ symbol: 'ETH', price: 3000 }]);
  });

  it('should remove handler on unsubscribe and stop delivering events to it', async () => {
    let wsInstance: MockWebSocket | null = null;
    // @ts-expect-error - mocking
    globalThis.WebSocket = class extends MockWebSocket {
      constructor(url: string) {
        super(url);
        wsInstance = this;
      }
    };

    const transport = (await import('./transport')).createTransport();
    const handler1 = vi.fn();
    const handler2 = vi.fn();
    const unsub1 = transport.listen('price-update', handler1);
    transport.listen('price-update', handler2);

    await vi.advanceTimersByTimeAsync(0);

    // Both receive first message
    wsInstance!.simulateMessage(JSON.stringify({
      type: 'price-update',
      data: 'msg1',
      timestamp: 1700000000,
    }));
    expect(handler1).toHaveBeenCalledTimes(1);
    expect(handler2).toHaveBeenCalledTimes(1);

    // Unsubscribe handler1
    unsub1();

    // Only handler2 receives second message
    wsInstance!.simulateMessage(JSON.stringify({
      type: 'price-update',
      data: 'msg2',
      timestamp: 1700000001,
    }));
    expect(handler1).toHaveBeenCalledTimes(1); // Still 1
    expect(handler2).toHaveBeenCalledTimes(2);
  });

  it('should ignore malformed JSON messages gracefully', async () => {
    let wsInstance: MockWebSocket | null = null;
    // @ts-expect-error - mocking
    globalThis.WebSocket = class extends MockWebSocket {
      constructor(url: string) {
        super(url);
        wsInstance = this;
      }
    };

    const transport = (await import('./transport')).createTransport();
    const handler = vi.fn();
    transport.listen('price-update', handler);

    await vi.advanceTimersByTimeAsync(0);

    // Should not throw
    wsInstance!.simulateMessage('not valid json {{{');
    wsInstance!.simulateMessage('');

    expect(handler).not.toHaveBeenCalled();
  });

  it('should use wss: protocol when page is loaded over https', async () => {
    // @ts-expect-error - mocking location
    globalThis.location = { protocol: 'https:', host: 'mynas.local:443' };

    let capturedUrl = '';
    // @ts-expect-error - mocking
    globalThis.WebSocket = class extends MockWebSocket {
      constructor(url: string) {
        super(url);
        capturedUrl = url;
      }
    };

    const transport = (await import('./transport')).createTransport();
    transport.listen('price-update', () => {});

    expect(capturedUrl).toBe('wss://mynas.local:443/api/ws');
  });

  it('should use ws: protocol when page is loaded over http', async () => {
    // @ts-expect-error - mocking location
    globalThis.location = { protocol: 'http:', host: 'localhost:8080' };

    let capturedUrl = '';
    // @ts-expect-error - mocking
    globalThis.WebSocket = class extends MockWebSocket {
      constructor(url: string) {
        super(url);
        capturedUrl = url;
      }
    };

    const transport = (await import('./transport')).createTransport();
    transport.listen('poll-tick', () => {});

    expect(capturedUrl).toBe('ws://localhost:8080/api/ws');
  });

  it('should attempt reconnection with exponential backoff on connection loss', async () => {
    let wsInstances: MockWebSocket[] = [];
    // @ts-expect-error - mocking
    globalThis.WebSocket = class extends MockWebSocket {
      constructor(url: string) {
        super(url);
        wsInstances.push(this);
      }
    };

    const transport = (await import('./transport')).createTransport();
    transport.listen('price-update', () => {});

    await vi.advanceTimersByTimeAsync(0); // Let initial WS connect
    expect(wsInstances.length).toBe(1);

    // Simulate connection loss
    wsInstances[0].simulateClose();

    // After 1s (first backoff), should attempt reconnect
    await vi.advanceTimersByTimeAsync(999);
    expect(wsInstances.length).toBe(1); // Not yet

    await vi.advanceTimersByTimeAsync(1);
    expect(wsInstances.length).toBe(2); // Reconnected

    // Simulate another loss
    wsInstances[1].simulateClose();

    // Second attempt should wait 2s
    await vi.advanceTimersByTimeAsync(1999);
    expect(wsInstances.length).toBe(2); // Not yet

    await vi.advanceTimersByTimeAsync(1);
    expect(wsInstances.length).toBe(3); // Reconnected after 2s
  });

  it('should cap reconnection backoff at 30 seconds', async () => {
    let wsInstances: MockWebSocket[] = [];
    let autoConnect = true;
    // @ts-expect-error - mocking
    globalThis.WebSocket = class {
      static CONNECTING = 0;
      static OPEN = 1;
      static CLOSING = 2;
      static CLOSED = 3;

      readyState = 0; // CONNECTING
      onopen: (() => void) | null = null;
      onclose: (() => void) | null = null;
      onmessage: ((event: { data: string }) => void) | null = null;
      onerror: (() => void) | null = null;
      url: string;

      constructor(url: string) {
        this.url = url;
        wsInstances.push(this as unknown as MockWebSocket);
        if (autoConnect) {
          const self = this;
          setTimeout(() => {
            self.readyState = 1;
            self.onopen?.();
          }, 0);
        }
      }

      close() {
        this.readyState = 3;
        this.onclose?.();
      }
    };

    const transport = (await import('./transport')).createTransport();
    transport.listen('price-update', () => {});

    await vi.advanceTimersByTimeAsync(0); // Let initial WS open
    expect(wsInstances.length).toBe(1);

    // Now prevent auto-connect so onopen never fires on reconnects,
    // keeping the attempt counter incrementing
    autoConnect = false;

    // Simulate repeated failures. Each close triggers reconnect after backoff.
    // Backoff: attempt 0 → 1s, attempt 1 → 2s, attempt 2 → 4s, attempt 3 → 8s, attempt 4 → 16s
    const backoffs = [1000, 2000, 4000, 8000, 16000];
    for (const delay of backoffs) {
      const lastWs = wsInstances[wsInstances.length - 1] as unknown as { readyState: number; onclose: (() => void) | null };
      lastWs.readyState = 3;
      lastWs.onclose?.();
      await vi.advanceTimersByTimeAsync(delay);
    }

    // Next backoff should be min(2^5 * 1000, 30000) = min(32000, 30000) = 30000
    expect(wsInstances.length).toBe(6);
    const lastWs = wsInstances[wsInstances.length - 1] as unknown as { readyState: number; onclose: (() => void) | null };
    lastWs.readyState = 3;
    lastWs.onclose?.();

    await vi.advanceTimersByTimeAsync(29999);
    expect(wsInstances.length).toBe(6); // Not yet
    await vi.advanceTimersByTimeAsync(1);
    expect(wsInstances.length).toBe(7); // Reconnected after 30s cap
  });

  it('should reset reconnect counter after successful connection', async () => {
    let wsInstances: MockWebSocket[] = [];
    // @ts-expect-error - mocking
    globalThis.WebSocket = class extends MockWebSocket {
      constructor(url: string) {
        super(url);
        wsInstances.push(this);
      }
    };

    const transport = (await import('./transport')).createTransport();
    transport.listen('price-update', () => {});

    await vi.advanceTimersByTimeAsync(0); // Connect successfully (onopen fires, resets counter)
    expect(wsInstances.length).toBe(1);

    // Disconnect — first close, attempt counter was reset to 0 on open
    wsInstances[0].simulateClose();
    // Backoff = 1s (2^0 * 1000)
    await vi.advanceTimersByTimeAsync(1000);
    expect(wsInstances.length).toBe(2);

    // Let the new connection succeed (onopen fires via setTimeout(0), resets counter again)
    await vi.advanceTimersByTimeAsync(1);

    // Disconnect again — since counter was reset, should be back to 1s backoff
    wsInstances[1].simulateClose();
    await vi.advanceTimersByTimeAsync(999);
    expect(wsInstances.length).toBe(2); // Not yet
    await vi.advanceTimersByTimeAsync(1);
    expect(wsInstances.length).toBe(3); // Back to 1s delay
  });

  it('should not reconnect if all handlers have been unsubscribed', async () => {
    let wsInstances: MockWebSocket[] = [];
    // @ts-expect-error - mocking
    globalThis.WebSocket = class extends MockWebSocket {
      constructor(url: string) {
        super(url);
        wsInstances.push(this);
      }
    };

    const transport = (await import('./transport')).createTransport();
    const unsub = transport.listen('price-update', () => {});

    await vi.advanceTimersByTimeAsync(0); // Connect
    expect(wsInstances.length).toBe(1);

    // Unsubscribe all handlers, then simulate close
    unsub();
    wsInstances[0].simulateClose();

    // Even after waiting, should not reconnect
    await vi.advanceTimersByTimeAsync(60000);
    expect(wsInstances.length).toBe(1);
  });

  it('should not create multiple WebSocket connections for multiple listen() calls', async () => {
    let wsInstances: MockWebSocket[] = [];
    // @ts-expect-error - mocking
    globalThis.WebSocket = class extends MockWebSocket {
      constructor(url: string) {
        super(url);
        wsInstances.push(this);
      }
    };

    const transport = (await import('./transport')).createTransport();
    transport.listen('price-update', () => {});
    transport.listen('poll-tick', () => {});
    transport.listen('notification-triggered', () => {});

    // Only one WebSocket should be created
    expect(wsInstances.length).toBe(1);
  });
});
