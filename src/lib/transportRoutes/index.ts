/**
 * HTTP route mapping for Tauri IPC command names.
 *
 * Maps each backend command to its corresponding REST endpoint
 * (method, path, optional JSON body).
 */

import type { RouteMapper } from './subscriptions';
import { subscriptionRoutes } from './subscriptions';
import { viewRoutes } from './views';
import { providerRoutes } from './providers';
import { notificationRoutes } from './notifications';
import { priceRoutes } from './prices';
import { systemRoutes } from './system';
import { dexRoutes } from './dex';
import { aiRoutes } from './ai';

const routes: Record<string, RouteMapper> = {
  ...subscriptionRoutes,
  ...viewRoutes,
  ...providerRoutes,
  ...notificationRoutes,
  ...priceRoutes,
  ...systemRoutes,
  ...dexRoutes,
  ...aiRoutes,
};

/**
 * Maps a Tauri IPC command name and its arguments to the corresponding
 * HTTP method, URL path, and optional JSON body for the REST API.
 *
 * Exported for unit testing and used by HttpTransport.
 */
export function mapCommandToHttp(
  command: string,
  args?: Record<string, unknown>
): { method: string; path: string; body?: string; extractField?: string } {
  const a = args ?? {};

  const mapper = routes[command];
  if (!mapper) {
    // Fallback for unknown commands — best-effort GET mapping
    return { method: 'GET', path: `/${command}` };
  }
  return mapper(a);
}
