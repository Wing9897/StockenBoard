import { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ProviderSettings, ProviderInfo } from '../types';
import { getDb } from '../lib/db';

export function useProviders() {
  const [settings, setSettings] = useState<Map<string, ProviderSettings>>(new Map());
  const [providerInfos, setProviderInfos] = useState<ProviderInfo[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadSettings();
    loadProviderInfos();
  }, []);

  async function loadSettings() {
    try {
      const db = await getDb();
      const rows = await db.select<ProviderSettings[]>(
        'SELECT provider_id, api_key, api_secret, refresh_interval, connection_type, enabled FROM provider_settings'
      );
      const map = new Map<string, ProviderSettings>();
      for (const row of rows) map.set(row.provider_id, row);
      setSettings(map);
    } catch (error) {
      console.error('Failed to load provider settings:', error);
    } finally {
      setLoading(false);
    }
  }

  async function loadProviderInfos() {
    try {
      setProviderInfos(await invoke<ProviderInfo[]>('get_all_providers'));
    } catch (error) {
      console.error('Failed to load provider infos:', error);
    }
  }

  async function updateProvider(providerId: string, updates: {
    api_key?: string | null;
    api_secret?: string | null;
    refresh_interval?: number;
    connection_type?: string;
    enabled?: number;
  }) {
    try {
      const db = await getDb();
      const current = settings.get(providerId);
      const enabled = updates.enabled ?? current?.enabled ?? 1;

      await db.execute(
        `INSERT INTO provider_settings (provider_id, api_key, api_secret, refresh_interval, connection_type, enabled)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT(provider_id) DO UPDATE SET
           api_key = $2, api_secret = $3, refresh_interval = $4, connection_type = $5, enabled = $6`,
        [
          providerId,
          updates.api_key || null,
          updates.api_secret || null,
          updates.refresh_interval || null,
          updates.connection_type || 'rest',
          enabled,
        ]
      );
      // 同步 Rust 端 provider instance + 觸發 polling reload
      if (enabled === 1) {
        await invoke('enable_provider', {
          providerId,
          apiKey: updates.api_key || null,
          apiSecret: updates.api_secret || null,
        });
      } else {
        await invoke('disable_provider', { providerId });
      }
      await loadSettings();
    } catch (error) {
      console.error('Failed to update provider:', error);
    }
  }

  async function toggleProvider(providerId: string, enabled: boolean) {
    try {
      const db = await getDb();
      await db.execute(
        'UPDATE provider_settings SET enabled = $1 WHERE provider_id = $2',
        [enabled ? 1 : 0, providerId]
      );
      if (enabled) {
        const s = settings.get(providerId);
        await invoke('enable_provider', {
          providerId,
          apiKey: s?.api_key || null,
          apiSecret: s?.api_secret || null,
        });
      } else {
        await invoke('disable_provider', { providerId });
      }
      await loadSettings();
    } catch (error) {
      console.error('Failed to toggle provider:', error);
    }
  }

  function getProviderInfo(providerId: string): ProviderInfo | undefined {
    return providerInfos.find(p => p.id === providerId);
  }

  const providers = useMemo(() => providerInfos.map(info => {
    const s = settings.get(info.id);
    return {
      id: info.id,
      name: info.name,
      provider_type: info.provider_type,
      api_key: s?.api_key || undefined,
      api_secret: s?.api_secret || undefined,
      refresh_interval: s?.refresh_interval ?? (s?.api_key ? info.key_interval : info.free_interval),
      connection_type: s?.connection_type || 'rest',
      supports_websocket: info.supports_websocket ? 1 : 0,
      enabled: s?.enabled ?? 1,
    };
  }), [providerInfos, settings]);

  return { providers, loading, updateProvider, toggleProvider, getProviderInfo, refresh: loadSettings };
}
