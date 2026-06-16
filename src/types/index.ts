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

/** 共用 Toast 操作介面 — 消除各 hook 重複定義的 ToastLike */
export interface ToastActions {
  success: (title: string, msg?: string) => void;
  error: (title: string, msg?: string) => void;
  info: (title: string, msg?: string) => void;
  warning?: (title: string, msg?: string) => void;
}

/** 通知通道（對應後端 list_notification_channels 回傳） */
export interface ChannelRow {
  id: number;
  channel_type: string;   // 'telegram' | 'webhook' | 'local'
  name: string;
  config: string;         // JSON string (empty '{}' for local)
  created_at: number;
}

/** 通知規則完整列（對應後端 list_notification_rules 回傳） */
export interface NotificationRuleRow {
  id: number;
  name: string;
  subscription_id: number;
  condition_type: string;
  threshold: number;
  channel_ids: string;    // JSON 陣列字串
  cooldown_secs: number;
  enabled: boolean;
  ai_config: string | null;
  created_at: number;
  updated_at: number;
}

/** 編輯規則表單所需的資料（NotificationRuleRow 的子集） */
export type EditRuleData = Pick<
  NotificationRuleRow,
  'id' | 'name' | 'subscription_id' | 'condition_type'
  | 'threshold' | 'channel_ids' | 'cooldown_secs' | 'ai_config'
>;

/** 通知規則觸發事件（後端 'notification-triggered' Tauri 事件 payload） */
export interface NotificationTriggeredEvent {
  rule_name: string;
  symbol: string;
  provider: string;
  price: number;
  condition_type: string;   // 'price_above' | 'price_below' | 'change_pct_above' | 'change_pct_below' | 'ai'
  threshold: number;
  triggered_at: number;     // Unix 秒
  is_ai: boolean;
  ai_reason: string | null;
}
