import { describe, it, expect, beforeEach, vi } from 'vitest';
import { priceStore } from './priceStore';
import type { AssetData } from '../types';

// Characterization tests for the global price store singleton.
// It drives every AssetCard via key-based subscriptions, so its change-detection
// and listener semantics are correctness-critical. Pure TS, no React/IPC.

function makeAsset(overrides: Partial<AssetData> & Pick<AssetData, 'symbol' | 'provider_id' | 'price'>): AssetData {
  return {
    currency: 'USD',
    change_24h: null,
    change_percent_24h: null,
    high_24h: null,
    low_24h: null,
    volume: null,
    market_cap: null,
    last_updated: 1000,
    extra: null,
    ...overrides,
  } as AssetData;
}

// The store is a module-level singleton; reset state between tests.
beforeEach(() => {
  priceStore.clear();
});

describe('priceStore.updatePrices + getAsset', () => {
  it('stores assets under a "provider:symbol" key', () => {
    priceStore.updatePrices([makeAsset({ symbol: 'BTC', provider_id: 'binance', price: 100 })]);
    expect(priceStore.getAsset('binance:BTC')?.price).toBe(100);
    expect(priceStore.getAsset('coinbase:BTC')).toBeUndefined();
  });

  it('notifies a key listener when price changes', () => {
    const fn = vi.fn();
    priceStore.subscribeKey('binance:BTC', fn);
    priceStore.updatePrices([makeAsset({ symbol: 'BTC', provider_id: 'binance', price: 100 })]);
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it('does NOT notify when neither price nor last_updated changed', () => {
    priceStore.updatePrices([makeAsset({ symbol: 'BTC', provider_id: 'binance', price: 100, last_updated: 5 })]);
    const fn = vi.fn();
    priceStore.subscribeKey('binance:BTC', fn);
    // Same price + same last_updated → no change → no notify.
    priceStore.updatePrices([makeAsset({ symbol: 'BTC', provider_id: 'binance', price: 100, last_updated: 5 })]);
    expect(fn).not.toHaveBeenCalled();
  });

  it('notifies when last_updated changes even if price is identical', () => {
    priceStore.updatePrices([makeAsset({ symbol: 'BTC', provider_id: 'binance', price: 100, last_updated: 5 })]);
    const fn = vi.fn();
    priceStore.subscribeKey('binance:BTC', fn);
    priceStore.updatePrices([makeAsset({ symbol: 'BTC', provider_id: 'binance', price: 100, last_updated: 6 })]);
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it('clears a stale error for the key on a successful price update and notifies', () => {
    priceStore.updateErrors({ 'binance:BTC': 'boom' });
    expect(priceStore.getError('binance:BTC')).toBe('boom');
    const fn = vi.fn();
    priceStore.subscribeKey('binance:BTC', fn);
    priceStore.updatePrices([makeAsset({ symbol: 'BTC', provider_id: 'binance', price: 100 })]);
    expect(priceStore.getError('binance:BTC')).toBeUndefined();
    expect(fn).toHaveBeenCalledTimes(1);
  });
});

describe('priceStore.updateErrors', () => {
  it('stores and notifies on a new error', () => {
    const fn = vi.fn();
    priceStore.subscribeKey('binance:BTC', fn);
    priceStore.updateErrors({ 'binance:BTC': 'rate limited' });
    expect(priceStore.getError('binance:BTC')).toBe('rate limited');
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it('does not notify when the same error message repeats', () => {
    priceStore.updateErrors({ 'binance:BTC': 'rate limited' });
    const fn = vi.fn();
    priceStore.subscribeKey('binance:BTC', fn);
    priceStore.updateErrors({ 'binance:BTC': 'rate limited' });
    expect(fn).not.toHaveBeenCalled();
  });
});

describe('priceStore.updateTick + getTick', () => {
  it('stores tick info and notifies tick listeners', () => {
    const fn = vi.fn();
    priceStore.subscribeTick('binance', fn);
    priceStore.updateTick('binance', 123, 5000);
    expect(priceStore.getTick('binance')).toEqual({ fetchedAt: 123, intervalMs: 5000 });
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it('does not notify when fetchedAt and intervalMs are unchanged', () => {
    priceStore.updateTick('binance', 123, 5000);
    const fn = vi.fn();
    priceStore.subscribeTick('binance', fn);
    priceStore.updateTick('binance', 123, 5000);
    expect(fn).not.toHaveBeenCalled();
  });
});

describe('priceStore.updateWs', () => {
  it('always stores and notifies (WS updates are authoritative)', () => {
    const fn = vi.fn();
    priceStore.subscribeKey('binance:ETH', fn);
    const data = makeAsset({ symbol: 'ETH', provider_id: 'binance', price: 50 });
    priceStore.updateWs('binance', 'ETH', data);
    expect(priceStore.getAsset('binance:ETH')?.price).toBe(50);
    expect(fn).toHaveBeenCalledTimes(1);
  });
});

describe('priceStore subscription lifecycle', () => {
  it('stops notifying after the unsubscribe fn is called', () => {
    const fn = vi.fn();
    const unsub = priceStore.subscribeKey('binance:BTC', fn);
    priceStore.updatePrices([makeAsset({ symbol: 'BTC', provider_id: 'binance', price: 1 })]);
    expect(fn).toHaveBeenCalledTimes(1);
    unsub();
    priceStore.updatePrices([makeAsset({ symbol: 'BTC', provider_id: 'binance', price: 2 })]);
    expect(fn).toHaveBeenCalledTimes(1); // no further calls
  });

  it('supports multiple listeners on the same key', () => {
    const a = vi.fn();
    const b = vi.fn();
    priceStore.subscribeKey('binance:BTC', a);
    priceStore.subscribeKey('binance:BTC', b);
    priceStore.updatePrices([makeAsset({ symbol: 'BTC', provider_id: 'binance', price: 1 })]);
    expect(a).toHaveBeenCalledTimes(1);
    expect(b).toHaveBeenCalledTimes(1);
  });

  it('clear() wipes assets, errors, and ticks', () => {
    priceStore.updatePrices([makeAsset({ symbol: 'BTC', provider_id: 'binance', price: 1 })]);
    priceStore.updateErrors({ 'x:Y': 'err' });
    priceStore.updateTick('binance', 1, 1000);
    priceStore.clear();
    expect(priceStore.getAsset('binance:BTC')).toBeUndefined();
    expect(priceStore.getError('x:Y')).toBeUndefined();
    expect(priceStore.getTick('binance')).toBeUndefined();
  });
});
