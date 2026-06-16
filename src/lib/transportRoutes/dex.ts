/**
 * DEX route mappings.
 */

import type { RouteMapper } from './subscriptions';

export const dexRoutes: Record<string, RouteMapper> = {
  lookup_dex_pool: (a) => ({
    method: 'GET',
    path: `/dex/pool/${encodeURIComponent(String(a.providerId ?? a.provider))}/${encodeURIComponent(String(a.poolAddress ?? a.address))}`,
  }),
};
