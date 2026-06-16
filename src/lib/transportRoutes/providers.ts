/**
 * Provider & provider-settings route mappings.
 */

import type { RouteMapper } from './subscriptions';

export const providerRoutes: Record<string, RouteMapper> = {
  get_all_providers: () => ({ method: 'GET', path: '/providers' }),
  enable_provider: (a) => ({
    method: 'POST',
    path: `/providers/${encodeURIComponent(String(a.id))}/enable`,
    body: JSON.stringify(a),
  }),
  list_provider_settings: () => ({ method: 'GET', path: '/provider-settings' }),
  upsert_provider_settings: (a) => ({
    method: 'PUT',
    path: `/provider-settings/${encodeURIComponent(String(a.providerId))}`,
    body: JSON.stringify({
      api_key: a.apiKey,
      api_secret: a.apiSecret,
      api_url: a.apiUrl,
      refresh_interval: a.refreshInterval,
      connection_type: a.connectionType,
      record_from_hour: a.recordFromHour,
      record_to_hour: a.recordToHour,
    }),
  }),
  has_api_key: (a) => ({
    method: 'GET',
    path: `/provider-settings/${encodeURIComponent(String(a.providerId))}/has-key`,
  }),
  set_provider_record_hours: (a) => ({
    method: 'PUT',
    path: `/provider-settings/${encodeURIComponent(String(a.provider_id ?? a.providerId))}/record-hours`,
    body: JSON.stringify({ from_hour: a.from_hour ?? a.fromHour, to_hour: a.to_hour ?? a.toHour }),
  }),
};
