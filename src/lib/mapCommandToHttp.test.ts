import { describe, it, expect } from 'vitest';
import { mapCommandToHttp } from './transport';

describe('mapCommandToHttp', () => {
  // --- Subscriptions ---
  describe('Subscriptions', () => {
    it('list_all_subscriptions → GET /subscriptions', () => {
      const result = mapCommandToHttp('list_all_subscriptions');
      expect(result).toEqual({ method: 'GET', path: '/subscriptions' });
    });

    it('list_subscriptions → GET /subscriptions?type=...', () => {
      const result = mapCommandToHttp('list_subscriptions', { type: 'asset' });
      expect(result).toEqual({ method: 'GET', path: '/subscriptions?type=asset' });
    });

    it('list_subscriptions with no type defaults to empty string', () => {
      const result = mapCommandToHttp('list_subscriptions', {});
      expect(result).toEqual({ method: 'GET', path: '/subscriptions?type=' });
    });

    it('add_subscription → POST /subscriptions with body', () => {
      const args = { symbol: 'BTC', provider: 'binance' };
      const result = mapCommandToHttp('add_subscription', args);
      expect(result.method).toBe('POST');
      expect(result.path).toBe('/subscriptions');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('add_subscriptions_batch → POST /subscriptions/batch with body', () => {
      const args = { subscriptions: [{ symbol: 'BTC' }, { symbol: 'ETH' }] };
      const result = mapCommandToHttp('add_subscriptions_batch', args);
      expect(result.method).toBe('POST');
      expect(result.path).toBe('/subscriptions/batch');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('update_subscription → PUT /subscriptions/:id with body', () => {
      const args = { id: 'sub-123', symbol: 'ETH' };
      const result = mapCommandToHttp('update_subscription', args);
      expect(result.method).toBe('PUT');
      expect(result.path).toBe('/subscriptions/sub-123');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('remove_subscription → DELETE /subscriptions/:id', () => {
      const result = mapCommandToHttp('remove_subscription', { id: 'sub-456' });
      expect(result.method).toBe('DELETE');
      expect(result.path).toBe('/subscriptions/sub-456');
      expect(result.body).toBeUndefined();
    });

    it('remove_subscriptions → DELETE /subscriptions/batch with body', () => {
      const args = { ids: ['sub-1', 'sub-2'] };
      const result = mapCommandToHttp('remove_subscriptions', args);
      expect(result.method).toBe('DELETE');
      expect(result.path).toBe('/subscriptions/batch');
      expect(result.body).toBe(JSON.stringify(args));
    });
  });

  // --- Views ---
  describe('Views', () => {
    it('list_views → GET /views?type=...', () => {
      const result = mapCommandToHttp('list_views', { type: 'asset' });
      expect(result).toEqual({ method: 'GET', path: '/views?type=asset' });
    });

    it('list_views with no type defaults to empty string', () => {
      const result = mapCommandToHttp('list_views', {});
      expect(result).toEqual({ method: 'GET', path: '/views?type=' });
    });

    it('create_view → POST /views with body', () => {
      const args = { name: 'My Portfolio', type: 'asset' };
      const result = mapCommandToHttp('create_view', args);
      expect(result.method).toBe('POST');
      expect(result.path).toBe('/views');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('rename_view → PUT /views/:id with body', () => {
      const args = { id: 'view-1', name: 'Renamed' };
      const result = mapCommandToHttp('rename_view', args);
      expect(result.method).toBe('PUT');
      expect(result.path).toBe('/views/view-1');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('delete_view → DELETE /views/:id', () => {
      const result = mapCommandToHttp('delete_view', { id: 'view-2' });
      expect(result.method).toBe('DELETE');
      expect(result.path).toBe('/views/view-2');
      expect(result.body).toBeUndefined();
    });

    it('add_sub_to_view → POST /views/:id/subscriptions with body', () => {
      const args = { id: 'view-1', sub_id: 'sub-99' };
      const result = mapCommandToHttp('add_sub_to_view', args);
      expect(result.method).toBe('POST');
      expect(result.path).toBe('/views/view-1/subscriptions');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('remove_sub_from_view → DELETE /views/:view_id/subscriptions/:sub_id', () => {
      const args = { view_id: 'view-1', sub_id: 'sub-99' };
      const result = mapCommandToHttp('remove_sub_from_view', args);
      expect(result.method).toBe('DELETE');
      expect(result.path).toBe('/views/view-1/subscriptions/sub-99');
      expect(result.body).toBeUndefined();
    });
  });

  // --- Providers ---
  describe('Providers', () => {
    it('get_all_providers → GET /providers', () => {
      const result = mapCommandToHttp('get_all_providers');
      expect(result).toEqual({ method: 'GET', path: '/providers' });
    });

    it('enable_provider → POST /providers/:id/enable with body', () => {
      const args = { id: 'binance', enabled: true };
      const result = mapCommandToHttp('enable_provider', args);
      expect(result.method).toBe('POST');
      expect(result.path).toBe('/providers/binance/enable');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('list_provider_settings → GET /provider-settings', () => {
      const result = mapCommandToHttp('list_provider_settings');
      expect(result).toEqual({ method: 'GET', path: '/provider-settings' });
    });

    it('upsert_provider_settings → PUT /provider-settings/:id with body', () => {
      const args = { id: 'coinbase', api_key: 'abc123' };
      const result = mapCommandToHttp('upsert_provider_settings', args);
      expect(result.method).toBe('PUT');
      expect(result.path).toBe('/provider-settings/coinbase');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('has_api_key → GET /provider-settings/:id/has-key', () => {
      const result = mapCommandToHttp('has_api_key', { id: 'binance' });
      expect(result.method).toBe('GET');
      expect(result.path).toBe('/provider-settings/binance/has-key');
      expect(result.body).toBeUndefined();
    });
  });

  // --- Notifications ---
  describe('Notifications', () => {
    it('create_notification_rule → POST /notifications/rules', () => {
      const args = { name: 'Price alert', condition: 'above' };
      const result = mapCommandToHttp('create_notification_rule', args);
      expect(result.method).toBe('POST');
      expect(result.path).toBe('/notifications/rules');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('list_notification_rules → GET /notifications/rules', () => {
      const result = mapCommandToHttp('list_notification_rules');
      expect(result).toEqual({ method: 'GET', path: '/notifications/rules' });
    });

    it('update_notification_rule → PUT /notifications/rules/:id', () => {
      const args = { id: 'rule-1', name: 'Updated' };
      const result = mapCommandToHttp('update_notification_rule', args);
      expect(result.method).toBe('PUT');
      expect(result.path).toBe('/notifications/rules/rule-1');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('delete_notification_rule → DELETE /notifications/rules/:id', () => {
      const result = mapCommandToHttp('delete_notification_rule', { id: 'rule-2' });
      expect(result.method).toBe('DELETE');
      expect(result.path).toBe('/notifications/rules/rule-2');
      expect(result.body).toBeUndefined();
    });

    it('toggle_notification_rule → POST /notifications/rules/:id/toggle', () => {
      const args = { id: 'rule-3', enabled: true };
      const result = mapCommandToHttp('toggle_notification_rule', args);
      expect(result.method).toBe('POST');
      expect(result.path).toBe('/notifications/rules/rule-3/toggle');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('save_notification_channel → POST /notifications/channels', () => {
      const args = { type: 'telegram', config: { bot_token: 'xyz' } };
      const result = mapCommandToHttp('save_notification_channel', args);
      expect(result.method).toBe('POST');
      expect(result.path).toBe('/notifications/channels');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('list_notification_channels → GET /notifications/channels', () => {
      const result = mapCommandToHttp('list_notification_channels');
      expect(result).toEqual({ method: 'GET', path: '/notifications/channels' });
    });

    it('delete_notification_channel → DELETE /notifications/channels/:id', () => {
      const result = mapCommandToHttp('delete_notification_channel', { id: 'ch-1' });
      expect(result.method).toBe('DELETE');
      expect(result.path).toBe('/notifications/channels/ch-1');
      expect(result.body).toBeUndefined();
    });

    it('test_notification_channel → POST /notifications/channels/:id/test', () => {
      const args = { id: 'ch-2', message: 'hello' };
      const result = mapCommandToHttp('test_notification_channel', args);
      expect(result.method).toBe('POST');
      expect(result.path).toBe('/notifications/channels/ch-2/test');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('get_notification_history → GET /notifications/history', () => {
      const result = mapCommandToHttp('get_notification_history');
      expect(result).toEqual({ method: 'GET', path: '/notifications/history' });
    });

    it('get_notification_global_cooldown → GET /notifications/cooldown', () => {
      const result = mapCommandToHttp('get_notification_global_cooldown');
      expect(result).toEqual({ method: 'GET', path: '/notifications/cooldown' });
    });

    it('set_notification_global_cooldown → PUT /notifications/cooldown', () => {
      const args = { seconds: 300 };
      const result = mapCommandToHttp('set_notification_global_cooldown', args);
      expect(result.method).toBe('PUT');
      expect(result.path).toBe('/notifications/cooldown');
      expect(result.body).toBe(JSON.stringify(args));
    });
  });

  // --- AI ---
  describe('AI', () => {
    it('save_ai_provider_config → POST /ai/config', () => {
      const args = { provider: 'openai', api_key: 'sk-xxx' };
      const result = mapCommandToHttp('save_ai_provider_config', args);
      expect(result.method).toBe('POST');
      expect(result.path).toBe('/ai/config');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('get_ai_provider_config → GET /ai/config', () => {
      const result = mapCommandToHttp('get_ai_provider_config');
      expect(result).toEqual({ method: 'GET', path: '/ai/config' });
    });

    it('test_ai_connection → POST /ai/test', () => {
      const args = { provider: 'openai' };
      const result = mapCommandToHttp('test_ai_connection', args);
      expect(result.method).toBe('POST');
      expect(result.path).toBe('/ai/test');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('list_ai_models → GET /ai/models', () => {
      const result = mapCommandToHttp('list_ai_models');
      expect(result).toEqual({ method: 'GET', path: '/ai/models' });
    });
  });

  // --- Prices ---
  describe('Prices', () => {
    it('fetch_asset_price → GET /prices/fetch/:provider/:symbol', () => {
      const result = mapCommandToHttp('fetch_asset_price', { provider: 'binance', symbol: 'BTCUSDT' });
      expect(result.method).toBe('GET');
      expect(result.path).toBe('/prices/fetch/binance/BTCUSDT');
      expect(result.body).toBeUndefined();
    });

    it('fetch_multiple_prices → POST /prices/fetch-multiple', () => {
      const args = { pairs: [{ provider: 'binance', symbol: 'BTC' }] };
      const result = mapCommandToHttp('fetch_multiple_prices', args);
      expect(result.method).toBe('POST');
      expect(result.path).toBe('/prices/fetch-multiple');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('get_cached_prices → GET /prices/cached', () => {
      const result = mapCommandToHttp('get_cached_prices');
      expect(result).toEqual({ method: 'GET', path: '/prices/cached' });
    });

    it('get_poll_ticks → GET /prices/poll-ticks', () => {
      const result = mapCommandToHttp('get_poll_ticks');
      expect(result).toEqual({ method: 'GET', path: '/prices/poll-ticks' });
    });
  });

  // --- History ---
  describe('History', () => {
    it('get_price_history → GET /history/:sub_id', () => {
      const result = mapCommandToHttp('get_price_history', { sub_id: 'sub-abc' });
      expect(result.method).toBe('GET');
      expect(result.path).toBe('/history/sub-abc');
      expect(result.body).toBeUndefined();
    });

    it('get_history_stats → GET /history/stats', () => {
      const result = mapCommandToHttp('get_history_stats');
      expect(result).toEqual({ method: 'GET', path: '/history/stats' });
    });

    it('cleanup_history → POST /history/cleanup', () => {
      const args = { older_than_days: 30 };
      const result = mapCommandToHttp('cleanup_history', args);
      expect(result.method).toBe('POST');
      expect(result.path).toBe('/history/cleanup');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('purge_all_history → DELETE /history', () => {
      const result = mapCommandToHttp('purge_all_history');
      expect(result).toEqual({ method: 'DELETE', path: '/history' });
    });

    it('delete_subscription_history → DELETE /history/:sub_id', () => {
      const result = mapCommandToHttp('delete_subscription_history', { sub_id: 'sub-xyz' });
      expect(result.method).toBe('DELETE');
      expect(result.path).toBe('/history/sub-xyz');
      expect(result.body).toBeUndefined();
    });
  });

  // --- Data ---
  describe('Data', () => {
    it('export_data → GET /data/export', () => {
      const result = mapCommandToHttp('export_data');
      expect(result).toEqual({ method: 'GET', path: '/data/export' });
    });

    it('import_data → POST /data/import', () => {
      const args = { data: { subscriptions: [] } };
      const result = mapCommandToHttp('import_data', args);
      expect(result.method).toBe('POST');
      expect(result.path).toBe('/data/import');
      expect(result.body).toBe(JSON.stringify(args));
    });
  });

  // --- System ---
  describe('System', () => {
    it('get_api_port → GET /system/config', () => {
      const result = mapCommandToHttp('get_api_port');
      expect(result).toEqual({ method: 'GET', path: '/system/config' });
    });

    it('set_api_port → PUT /system/config', () => {
      const args = { port: 9090 };
      const result = mapCommandToHttp('set_api_port', args);
      expect(result.method).toBe('PUT');
      expect(result.path).toBe('/system/config');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('get_unattended_polling → GET /system/config', () => {
      const result = mapCommandToHttp('get_unattended_polling');
      expect(result).toEqual({ method: 'GET', path: '/system/config' });
    });

    it('set_unattended_polling → PUT /system/config', () => {
      const args = { enabled: true };
      const result = mapCommandToHttp('set_unattended_polling', args);
      expect(result.method).toBe('PUT');
      expect(result.path).toBe('/system/config');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('reload_polling → POST /system/reload-polling', () => {
      const result = mapCommandToHttp('reload_polling');
      expect(result).toEqual({ method: 'POST', path: '/system/reload-polling' });
    });

    it('reset_all_data → POST /system/reset', () => {
      const result = mapCommandToHttp('reset_all_data');
      expect(result).toEqual({ method: 'POST', path: '/system/reset' });
    });

    it('get_data_dir → GET /system/data-dir', () => {
      const result = mapCommandToHttp('get_data_dir');
      expect(result).toEqual({ method: 'GET', path: '/system/data-dir' });
    });
  });

  // --- Icons ---
  describe('Icons', () => {
    it('set_icon → POST /icons/:symbol with body', () => {
      const args = { symbol: 'BTC', data: 'base64...' };
      const result = mapCommandToHttp('set_icon', args);
      expect(result.method).toBe('POST');
      expect(result.path).toBe('/icons/BTC');
      expect(result.body).toBe(JSON.stringify(args));
    });

    it('remove_icon → DELETE /icons/:symbol', () => {
      const result = mapCommandToHttp('remove_icon', { symbol: 'ETH' });
      expect(result.method).toBe('DELETE');
      expect(result.path).toBe('/icons/ETH');
      expect(result.body).toBeUndefined();
    });

    it('get_icons_dir → GET /icons', () => {
      const result = mapCommandToHttp('get_icons_dir');
      expect(result).toEqual({ method: 'GET', path: '/icons' });
    });

    it('download_logos → POST /icons/download-logos with body', () => {
      const args = { symbols: ['BTC', 'ETH'] };
      const result = mapCommandToHttp('download_logos', args);
      expect(result.method).toBe('POST');
      expect(result.path).toBe('/icons/download-logos');
      expect(result.body).toBe(JSON.stringify(args));
    });
  });

  // --- DEX ---
  describe('DEX', () => {
    it('lookup_dex_pool → GET /dex/pool/:provider/:address', () => {
      const result = mapCommandToHttp('lookup_dex_pool', {
        provider: 'uniswap',
        address: '0xabc123',
      });
      expect(result.method).toBe('GET');
      expect(result.path).toBe('/dex/pool/uniswap/0xabc123');
      expect(result.body).toBeUndefined();
    });
  });

  // --- Edge Cases ---
  describe('Edge cases', () => {
    it('unknown command falls back to GET /<command>', () => {
      const result = mapCommandToHttp('non_existent_command');
      expect(result).toEqual({ method: 'GET', path: '/non_existent_command' });
    });

    it('unknown command with args still uses fallback', () => {
      const result = mapCommandToHttp('unknown_cmd', { foo: 'bar' });
      expect(result).toEqual({ method: 'GET', path: '/unknown_cmd' });
    });

    it('command with no args (undefined) works correctly', () => {
      const result = mapCommandToHttp('list_all_subscriptions', undefined);
      expect(result).toEqual({ method: 'GET', path: '/subscriptions' });
    });

    it('command with empty args object works correctly', () => {
      const result = mapCommandToHttp('list_all_subscriptions', {});
      expect(result).toEqual({ method: 'GET', path: '/subscriptions' });
    });

    it('special characters in id are URL-encoded', () => {
      const result = mapCommandToHttp('remove_subscription', { id: 'a/b c&d=e' });
      expect(result.method).toBe('DELETE');
      expect(result.path).toBe(`/subscriptions/${encodeURIComponent('a/b c&d=e')}`);
    });

    it('special characters in provider and symbol are URL-encoded', () => {
      const result = mapCommandToHttp('fetch_asset_price', {
        provider: 'my provider',
        symbol: 'BTC/USD',
      });
      expect(result.path).toBe(
        `/prices/fetch/${encodeURIComponent('my provider')}/${encodeURIComponent('BTC/USD')}`
      );
    });

    it('special characters in view_id and sub_id are URL-encoded', () => {
      const result = mapCommandToHttp('remove_sub_from_view', {
        view_id: 'v/1',
        sub_id: 's&2',
      });
      expect(result.path).toBe(
        `/views/${encodeURIComponent('v/1')}/subscriptions/${encodeURIComponent('s&2')}`
      );
    });

    it('missing id argument results in "undefined" being encoded', () => {
      // When no id is provided, String(undefined) = "undefined"
      const result = mapCommandToHttp('remove_subscription', {});
      expect(result.method).toBe('DELETE');
      expect(result.path).toBe('/subscriptions/undefined');
    });
  });
});
