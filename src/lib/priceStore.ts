/**
 * 全域價格 store — key-based subscription 模式，
 * 每張卡片只在自己的 key 變化時收到通知，避免 O(N) 廣播。
 *
 * 從 useAssetData.ts 抽出，讓 data layer 與 React hook 解耦。
 */
import type { AssetData } from '../types';

class PriceStore {
  private assets = new Map<string, AssetData>();
  private errors = new Map<string, string>();
  private ticks = new Map<string, { fetchedAt: number; intervalMs: number }>();
  private keyListeners = new Map<string, Set<() => void>>();
  private tickListeners = new Map<string, Set<() => void>>();

  getAsset(key: string) { return this.assets.get(key); }
  getError(key: string) { return this.errors.get(key); }
  getTick(providerId: string) { return this.ticks.get(providerId); }

  updatePrices(results: AssetData[]) {
    for (const d of results) {
      const key = `${d.provider_id}:${d.symbol}`;
      const prev = this.assets.get(key);
      let changed = false;
      if (!prev || prev.price !== d.price || prev.last_updated !== d.last_updated) {
        this.assets.set(key, d);
        changed = true;
      }
      if (this.errors.has(key)) { this.errors.delete(key); changed = true; }
      if (changed) this.notifyKey(key);
    }
  }

  updateErrors(payload: Record<string, string>) {
    for (const [k, msg] of Object.entries(payload)) {
      if (this.errors.get(k) !== msg) { this.errors.set(k, msg); this.notifyKey(k); }
    }
  }

  updateTick(providerId: string, fetchedAt: number, intervalMs: number) {
    const prev = this.ticks.get(providerId);
    if (prev && prev.fetchedAt === fetchedAt && prev.intervalMs === intervalMs) return;
    this.ticks.set(providerId, { fetchedAt, intervalMs });
    this.notifyTick(providerId);
  }

  updateWs(providerId: string, symbol: string, data: AssetData) {
    const key = `${providerId}:${symbol}`;
    this.assets.set(key, data);
    this.notifyKey(key);
  }

  clear() { this.assets.clear(); this.errors.clear(); this.ticks.clear(); }

  subscribeKey(key: string, fn: () => void) {
    let set = this.keyListeners.get(key);
    if (!set) { set = new Set(); this.keyListeners.set(key, set); }
    set.add(fn);
    return () => { set!.delete(fn); if (set!.size === 0) this.keyListeners.delete(key); };
  }

  subscribeTick(providerId: string, fn: () => void) {
    let set = this.tickListeners.get(providerId);
    if (!set) { set = new Set(); this.tickListeners.set(providerId, set); }
    set.add(fn);
    return () => { set!.delete(fn); if (set!.size === 0) this.tickListeners.delete(providerId); };
  }

  private notifyKey(key: string) {
    const fns = this.keyListeners.get(key);
    if (fns) for (const fn of fns) fn();
  }
  private notifyTick(providerId: string) {
    const fns = this.tickListeners.get(providerId);
    if (fns) for (const fn of fns) fn();
  }
}

/** 全域單例 */
export const priceStore = new PriceStore();
