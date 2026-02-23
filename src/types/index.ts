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
  provider_type: 'crypto' | 'stock' | 'both' | 'prediction';
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
  refresh_interval?: number;
  connection_type: string;
  enabled: number;
}

export interface Subscription {
  id: number;
  symbol: string;
  display_name?: string;
  selected_provider_id: string;
  asset_type: 'crypto' | 'stock';
  sort_order: number;
}

export interface View {
  id: number;
  name: string;
  is_default: boolean;
}

export interface WsTickerUpdate {
  symbol: string;
  provider_id: string;
  data: AssetData;
}
