/**
 * Price & history route mappings.
 */

import type { RouteMapper } from './subscriptions';

export const priceRoutes: Record<string, RouteMapper> = {
  // --- Prices ---
  fetch_asset_price: (a) => ({
    method: 'GET',
    path: `/prices/fetch/${encodeURIComponent(String(a.providerId ?? a.provider))}/${encodeURIComponent(String(a.symbol))}`,
  }),
  fetch_multiple_prices: (a) => ({
    method: 'POST',
    path: '/prices/fetch-multiple',
    body: JSON.stringify({ provider_id: a.providerId, symbols: a.symbols }),
  }),
  get_cached_prices: () => ({ method: 'GET', path: '/prices/cached' }),
  get_poll_ticks: () => ({ method: 'GET', path: '/prices/poll-ticks' }),

  // --- History ---
  get_price_history: (a) => ({
    method: 'GET',
    path: `/history/${encodeURIComponent(String(a.subscriptionId))}?${new URLSearchParams({
      ...(a.fromTs != null ? { from: String(a.fromTs) } : {}),
      ...(a.toTs != null ? { to: String(a.toTs) } : {}),
      ...(a.limit != null ? { limit: String(a.limit) } : {}),
    }).toString()}`,
  }),
  get_history_stats: (a) => ({
    method: 'GET',
    path: `/history/stats${(a.subscriptionIds as number[] | undefined)?.length ? `?subscription_ids=${encodeURIComponent((a.subscriptionIds as number[]).join(','))}` : ''}`,
  }),
  cleanup_history: (a) => ({
    method: 'POST',
    path: '/history/cleanup',
    body: JSON.stringify({ retention_days: a.retentionDays ?? a.retention_days }),
    extractField: 'deleted',
  }),
  purge_all_history: () => ({ method: 'DELETE', path: '/history', extractField: 'deleted' }),
  delete_subscription_history: (a) => ({
    method: 'DELETE',
    path: `/history/${encodeURIComponent(String(a.subscriptionId))}`,
    extractField: 'deleted',
  }),
};
