/**
 * AI route mappings.
 */

import type { RouteMapper } from './subscriptions';

export const aiRoutes: Record<string, RouteMapper> = {
  save_ai_provider_config: (a) => ({
    method: 'POST',
    path: '/ai/config',
    body: JSON.stringify(a),
  }),
  get_ai_provider_config: () => ({ method: 'GET', path: '/ai/config' }),
  test_ai_connection: (a) => ({
    method: 'POST',
    path: '/ai/test',
    body: JSON.stringify({
      base_url: a.baseUrl ?? a.base_url,
      model: a.model,
      api_key: a.apiKey ?? a.api_key ?? null,
    }),
    extractField: 'message',
  }),
  list_ai_models: (a) => ({
    method: 'GET',
    path: `/ai/models?${new URLSearchParams({
      base_url: String(a.baseUrl ?? a.base_url ?? ''),
      ...(a.apiKey || a.api_key ? { api_key: String(a.apiKey ?? a.api_key) } : {}),
    }).toString()}`,
  }),
};
