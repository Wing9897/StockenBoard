export interface AssetData {
  symbol: string;
  price: number;
  currency: string;
  change_24h?: number;
  change_percent_24h?: number;
  high_24h?: number;
  low_24h?: number;
  volume?: number;
  market_cap?: number;
  last_updated: number;
  provider_id: string;
  extra?: Record<string, unknown>;
}

export interface ProviderInfo {
  id: string;
  name: string;
  provider_type: 'crypto' | 'stock' | 'both' | 'prediction' | 'dex';
  requires_api_key: boolean;
  requires_api_secret: boolean;
  supports_websocket: boolean;
  optional_api_key: boolean;
  free_tier_info: string;
  symbol_format: string;
  supported_fields: string[];
  free_interval: number;
  key_interval: number;
}

export interface ProviderSettings {
  provider_id: string;
  api_key?: string;
  api_secret?: string;
  api_url?: string;
  refresh_interval?: number;
  connection_type: string;
  record_from_hour?: number | null;
  record_to_hour?: number | null;
}

export interface Subscription {
  id: number;
  sub_type: 'asset' | 'dex';
  symbol: string;
  display_name?: string;
  selected_provider_id: string;
  asset_type: string;
  pool_address?: string;
  token_from_address?: string;
  token_to_address?: string;
  sort_order: number;
  record_enabled: number;
  record_from_hour?: number | null;
  record_to_hour?: number | null;
}

export interface View {
  id: number;
  name: string;
  view_type: 'asset' | 'dex';
  is_default: boolean;
}

export interface WsTickerUpdate {
  symbol: string;
  provider_id: string;
  data: AssetData;
}

export type ViewMode = 'grid' | 'list' | 'compact';

export interface PriceHistoryRecord {
  id: number;
  subscription_id: number;
  provider_id: string;
  price: number;
  change_pct: number | null;
  volume: number | null;
  pre_price: number | null;
  post_price: number | null;
  recorded_at: number;
}

export interface HistoryStats {
  subscription_id: number;
  total_records: number;
  earliest: number | null;
  latest: number | null;
}
