/// 單一乾淨 schema — 不需要與舊版相容
pub const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS provider_settings (
    provider_id TEXT PRIMARY KEY,
    api_key TEXT,
    api_secret TEXT,
    refresh_interval INTEGER,
    connection_type TEXT DEFAULT 'rest'
);

CREATE TABLE IF NOT EXISTS subscriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL UNIQUE,
    display_name TEXT,
    selected_provider_id TEXT NOT NULL DEFAULT 'binance',
    asset_type TEXT NOT NULL DEFAULT 'crypto'
);

CREATE TABLE IF NOT EXISTS views (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    is_default INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS view_subscriptions (
    view_id INTEGER NOT NULL,
    subscription_id INTEGER NOT NULL,
    PRIMARY KEY (view_id, subscription_id),
    FOREIGN KEY (view_id) REFERENCES views(id) ON DELETE CASCADE,
    FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
);

INSERT OR IGNORE INTO views (id, name, is_default) VALUES (1, '全部', 1);
"#;
