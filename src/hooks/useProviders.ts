import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import Database from '@tauri-apps/plugin-sql';
import { Provider, ProviderInfo } from '../types';

export function useProviders() {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [providerInfos, setProviderInfos] = useState<ProviderInfo[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadProviders();
    loadProviderInfos();
  }, []);

  async function loadProviders() {
    try {
      const db = await Database.load('sqlite:stockenboard.db');
      const result = await db.select<Provider[]>('SELECT * FROM providers ORDER BY name');
      setProviders(result);
    } catch (error) {
      console.error('Failed to load providers:', error);
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

  async function updateProvider(provider: Partial<Provider> & { id: string }) {
    try {
      const db = await Database.load('sqlite:stockenboard.db');

      // Only update the fields that were provided
      await db.execute(
        `UPDATE providers SET
          api_key = $1,
          api_secret = $2,
          refresh_interval = $3,
          connection_type = $4
        WHERE id = $5`,
        [
          provider.api_key || null,
          provider.api_secret || null,
          provider.refresh_interval || 30000,
          provider.connection_type || 'rest',
          provider.id,
        ]
      );

      // Always sync the provider to Rust side with latest API key
      // This ensures the in-memory provider instance has the correct credentials
      await invoke('enable_provider', {
        providerId: provider.id,
        apiKey: provider.api_key || null,
        apiSecret: provider.api_secret || null,
      });

      await loadProviders();
    } catch (error) {
      console.error('Failed to update provider:', error);
    }
  }

  function getProviderInfo(providerId: string): ProviderInfo | undefined {
    return providerInfos.find((p) => p.id === providerId);
  }

  return {
    providers,
    providerInfos,
    loading,
    updateProvider,
    getProviderInfo,
    refresh: loadProviders,
  };
}
