/// StockenBoard DB schema — 不需要向後兼容，刪除舊 DB 重新建立即可
pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS provider_settings (
    provider_id      TEXT PRIMARY KEY,
    api_key          TEXT,
    api_secret       TEXT,
    api_url          TEXT,
    refresh_interval INTEGER,
    connection_type  TEXT NOT NULL DEFAULT 'rest',
    enabled          INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS subscriptions (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    sub_type             TEXT NOT NULL DEFAULT 'asset',
    symbol               TEXT NOT NULL,
    display_name         TEXT,
    selected_provider_id TEXT NOT NULL DEFAULT 'binance',
    asset_type           TEXT NOT NULL DEFAULT 'crypto',
    pool_address         TEXT,
    token_from_address   TEXT,
    token_to_address     TEXT,
    sort_order           INTEGER NOT NULL DEFAULT 0,
    UNIQUE(symbol, selected_provider_id)
);

CREATE TABLE IF NOT EXISTS views (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL,
    view_type  TEXT NOT NULL DEFAULT 'asset',
    is_default INTEGER NOT NULL DEFAULT 0,
    UNIQUE(name, view_type)
);

CREATE TABLE IF NOT EXISTS view_subscriptions (
    view_id         INTEGER NOT NULL,
    subscription_id INTEGER NOT NULL,
    PRIMARY KEY (view_id, subscription_id),
    FOREIGN KEY (view_id) REFERENCES views(id) ON DELETE CASCADE,
    FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
);

INSERT OR IGNORE INTO views (id, name, view_type, is_default) VALUES (1, '全部', 'asset', 1);
INSERT OR IGNORE INTO views (id, name, view_type, is_default) VALUES (2, '全部', 'dex', 1);

CREATE TRIGGER IF NOT EXISTS auto_sort_order
AFTER INSERT ON subscriptions
WHEN NEW.sort_order = 0
BEGIN
    UPDATE subscriptions SET sort_order = NEW.id WHERE id = NEW.id;
END;
"#;
