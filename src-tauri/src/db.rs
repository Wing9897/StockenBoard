/// 統一的初始化 SQL — 包含所有表結構與預設資料
pub const INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    api_key TEXT,
    api_secret TEXT,
    base_url TEXT,
    refresh_interval INTEGER DEFAULT 30000,
    enabled INTEGER DEFAULT 0,
    connection_type TEXT DEFAULT 'rest',
    supports_websocket INTEGER DEFAULT 0,
    config TEXT
);

CREATE TABLE IF NOT EXISTS subscriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL UNIQUE,
    display_name TEXT,
    icon_path TEXT,
    default_provider_id TEXT,
    selected_provider_id TEXT,
    asset_type TEXT DEFAULT 'crypto',
    sort_order INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS views (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    is_default INTEGER DEFAULT 0,
    sort_order INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS view_subscriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    view_id INTEGER NOT NULL,
    subscription_id INTEGER NOT NULL,
    sort_order INTEGER DEFAULT 0,
    FOREIGN KEY (view_id) REFERENCES views(id) ON DELETE CASCADE,
    FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE,
    UNIQUE(view_id, subscription_id)
);

-- 預設「全部」頁面
INSERT OR IGNORE INTO views (id, name, is_default, sort_order) VALUES (1, '全部', 1, 0);

INSERT OR IGNORE INTO providers (id, name, provider_type, refresh_interval, enabled, connection_type, supports_websocket) VALUES
    ('binance', 'Binance', 'crypto', 5000, 1, 'rest', 1),
    ('coinbase', 'Coinbase', 'crypto', 5000, 1, 'rest', 1),
    ('coingecko', 'CoinGecko', 'crypto', 60000, 1, 'rest', 0),
    ('coinmarketcap', 'CoinMarketCap', 'crypto', 60000, 1, 'rest', 0),
    ('cryptocompare', 'CryptoCompare', 'crypto', 30000, 1, 'rest', 1),
    ('yahoo', 'Yahoo Finance', 'stock', 15000, 1, 'rest', 0),
    ('marketstack', 'Marketstack', 'stock', 600000, 1, 'rest', 0),
    ('eodhd', 'EODHD', 'stock', 300000, 1, 'rest', 0),
    ('mboum', 'Mboum', 'stock', 60000, 1, 'rest', 0),
    ('alpaca', 'Alpaca', 'both', 5000, 1, 'rest', 1),
    ('finnhub', 'Finnhub', 'both', 10000, 1, 'rest', 1),
    ('alphavantage', 'Alpha Vantage', 'both', 180000, 1, 'rest', 0),
    ('polygon', 'Polygon.io', 'both', 60000, 1, 'rest', 1),
    ('tiingo', 'Tiingo', 'both', 120000, 1, 'rest', 0),
    ('fmp', 'Financial Modeling Prep', 'both', 360000, 1, 'rest', 0),
    ('twelvedata', 'Twelve Data', 'both', 15000, 1, 'rest', 1),
    ('polymarket', 'Polymarket', 'prediction', 5000, 1, 'rest', 1),
    ('bitquery', 'Bitquery', 'prediction', 30000, 1, 'rest', 0);
"#;
