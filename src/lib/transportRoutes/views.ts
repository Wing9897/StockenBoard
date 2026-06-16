/**
 * View route mappings.
 */

import type { RouteMapper } from './subscriptions';

export const viewRoutes: Record<string, RouteMapper> = {
  list_views: (a) => ({
    method: 'GET',
    path: `/views?type=${encodeURIComponent(String(a.viewType ?? ''))}`,
  }),
  create_view: (a) => ({
    method: 'POST',
    path: '/views',
    body: JSON.stringify({ name: a.name, type: a.viewType }),
  }),
  rename_view: (a) => ({
    method: 'PUT',
    path: `/views/${encodeURIComponent(String(a.id))}`,
    body: JSON.stringify(a),
  }),
  delete_view: (a) => ({
    method: 'DELETE',
    path: `/views/${encodeURIComponent(String(a.id))}`,
  }),
  add_sub_to_view: (a) => ({
    method: 'POST',
    path: `/views/${encodeURIComponent(String(a.viewId))}/subscriptions`,
    body: JSON.stringify({ subscription_id: a.subscriptionId }),
  }),
  remove_sub_from_view: (a) => ({
    method: 'DELETE',
    path: `/views/${encodeURIComponent(String(a.viewId))}/subscriptions/${encodeURIComponent(String(a.subscriptionId))}`,
  }),
  get_view_sub_counts: () => ({ method: 'GET', path: '/views/sub-counts' }),
  get_view_subscription_ids: (a) => ({
    method: 'GET',
    path: `/views/${encodeURIComponent(String(a.viewId))}/subscription-ids`,
  }),
};
