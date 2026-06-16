/**
 * Subscription route mappings.
 */

export type RouteMapper = (a: Record<string, unknown>) => {
  method: string;
  path: string;
  body?: string;
  extractField?: string;
};

export const subscriptionRoutes: Record<string, RouteMapper> = {
  list_all_subscriptions: () => ({ method: 'GET', path: '/subscriptions' }),
  list_subscriptions: (a) => ({
    method: 'GET',
    path: `/subscriptions?type=${encodeURIComponent(String(a.subType ?? ''))}`,
  }),
  add_subscription: (a) => ({
    method: 'POST',
    path: '/subscriptions',
    body: JSON.stringify({
      sub_type: a.subType,
      symbol: a.symbol,
      display_name: a.displayName,
      provider_id: a.providerId,
      asset_type: a.assetType,
      pool_address: a.poolAddress,
      token_from: a.tokenFrom,
      token_to: a.tokenTo,
    }),
  }),
  add_subscriptions_batch: (a) => ({
    method: 'POST',
    path: '/subscriptions/batch',
    body: JSON.stringify(
      (a.items as Array<Record<string, unknown>>).map((item) => ({
        sub_type: item.subType ?? item.sub_type,
        symbol: item.symbol,
        display_name: item.displayName ?? item.display_name,
        provider_id: item.providerId ?? item.provider_id,
        asset_type: item.assetType ?? item.asset_type,
        pool_address: item.poolAddress ?? item.pool_address,
        token_from: item.tokenFrom ?? item.token_from,
        token_to: item.tokenTo ?? item.token_to,
      }))
    ),
  }),
  update_subscription: (a) => ({
    method: 'PUT',
    path: `/subscriptions/${encodeURIComponent(String(a.id))}`,
    body: JSON.stringify({
      symbol: a.symbol,
      display_name: a.displayName,
      provider_id: a.providerId,
      asset_type: a.assetType,
    }),
  }),
  remove_subscription: (a) => ({
    method: 'DELETE',
    path: `/subscriptions/${encodeURIComponent(String(a.id))}`,
  }),
  remove_subscriptions: (a) => ({
    method: 'DELETE',
    path: '/subscriptions/batch',
    body: JSON.stringify(a),
  }),
  toggle_record: (a) => ({
    method: 'POST',
    path: `/subscriptions/${encodeURIComponent(String(a.subscriptionId))}/toggle-record`,
    body: JSON.stringify({
      enabled: a.enabled,
      ...(a.confirmed !== undefined ? { confirmed: a.confirmed } : {}),
    }),
  }),
  set_record_hours: (a) => ({
    method: 'PUT',
    path: `/subscriptions/${encodeURIComponent(String(a.subscriptionId))}/record-hours`,
    body: JSON.stringify({ from_hour: a.fromHour, to_hour: a.toHour }),
  }),
};
