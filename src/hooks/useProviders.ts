import { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import Database from '@tauri-apps/plugin-sql';
import { ProviderSettings, ProviderInfo } from '../types';

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
      const db = await Database.load('sqlite:stockenboard.db');
      const rows = await db.select<ProviderSettings[]>(
        'SELECT provider_id, api_key, api_secret, refresh_interval, connection_type FROM provider_settings'
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
      const result = await invoke<ProviderInfo[]>('get_all_providers');
      setProviderInfos(result);
    } catch (error) {
      console.error('Failed to load provider infos:', error);
    }
  }

  async function updateProvider(providerId: string, updates: {
    api_key?: string | null;
    api_secret?: string | null;
    refresh_interval?: number;
    connection_type?: string;
  }) {
    try {
      const db = await Database.load('sqlite:stockenboard.db');
      await db.execute(
        `INSERT INTO provider_settings (provider_id, api_key, api_secret, refresh_interval, connection_type)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT(provider_id) DO UPDATE SET
           api_key = $2, api_secret = $3, refresh_interval = $4, connection_type = $5`,
        [
          providerId,
          updates.api_key || null,
          updates.api_secret || null,
          updates.refresh_interval || null,
          updates.connection_type || 'rest',
        ]
      );

      // 同步 Rust 端的 provider 實例
      await invoke('enable_provider', {
        providerId,
        apiKey: updates.api_key || null,
        apiSecret: updates.api_secret || null,
      });

      await loadSettings();
    } catch (error) {
      console.error('Failed to update provider:', error);
    }
  }

  function getProviderInfo(providerId: string): ProviderInfo | undefined {
    return providerInfos.find(p => p.id === providerId);
  }

  // 合併靜態 ProviderInfo + 用戶設定，供 UI 使用 — memoized
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
    };
  }), [providerInfos, settings]);

  return {
    providers,
    loading,
    updateProvider,
    getProviderInfo,
    refresh: loadSettings,
  };
}
