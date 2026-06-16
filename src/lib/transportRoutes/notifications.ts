/**
 * Notification rules/channels/history/cooldown route mappings.
 */

import type { RouteMapper } from './subscriptions';

export const notificationRoutes: Record<string, RouteMapper> = {
  // --- Rules ---
  create_notification_rule: (a) => ({
    method: 'POST',
    path: '/notifications/rules',
    body: JSON.stringify(a.rule),
  }),
  list_notification_rules: () => ({ method: 'GET', path: '/notifications/rules' }),
  update_notification_rule: (a) => ({
    method: 'PUT',
    path: `/notifications/rules/${encodeURIComponent(String(a.id))}`,
    body: JSON.stringify(a.rule),
  }),
  delete_notification_rule: (a) => ({
    method: 'DELETE',
    path: `/notifications/rules/${encodeURIComponent(String(a.id))}`,
  }),
  toggle_notification_rule: (a) => ({
    method: 'POST',
    path: `/notifications/rules/${encodeURIComponent(String(a.id))}/toggle`,
    body: JSON.stringify({ enabled: a.enabled }),
  }),

  // --- Channels ---
  save_notification_channel: (a) => ({
    method: 'POST',
    path: '/notifications/channels',
    body: JSON.stringify(a.channel),
  }),
  list_notification_channels: () => ({ method: 'GET', path: '/notifications/channels' }),
  delete_notification_channel: (a) => ({
    method: 'DELETE',
    path: `/notifications/channels/${encodeURIComponent(String(a.id))}`,
  }),
  test_notification_channel: (a) => ({
    method: 'POST',
    path: `/notifications/channels/${encodeURIComponent(String(a.id))}/test`,
    body: JSON.stringify(a),
  }),

  // --- History & Cooldown ---
  get_notification_history: (a) => ({
    method: 'GET',
    path: `/notifications/history?${new URLSearchParams({
      ...(a.ruleId != null ? { rule_id: String(a.ruleId) } : {}),
      ...(a.from != null ? { from: String(a.from) } : {}),
      ...(a.to != null ? { to: String(a.to) } : {}),
      ...(a.limit != null ? { limit: String(a.limit) } : {}),
    }).toString()}`,
  }),
  get_notification_global_cooldown: () => ({ method: 'GET', path: '/notifications/cooldown' }),
  set_notification_global_cooldown: (a) => ({
    method: 'PUT',
    path: '/notifications/cooldown',
    body: JSON.stringify({ seconds: a.secs }),
  }),
};
