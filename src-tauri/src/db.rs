/// StockenBoard DB schema
pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS provider_settings (
    provider_id      TEXT PRIMARY KEY,
    api_key          TEXT,
    api_secret       TEXT,
    api_url          TEXT,
    refresh_interval INTEGER,
    connection_type  TEXT NOT NULL DEFAULT 'rest',
    record_from_hour INTEGER,
    record_to_hour   INTEGER
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
    record_enabled       INTEGER NOT NULL DEFAULT 0,
    record_from_hour     INTEGER,
    record_to_hour       INTEGER,
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

CREATE TABLE IF NOT EXISTS price_history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    subscription_id INTEGER NOT NULL,
    provider_id     TEXT NOT NULL,
    price           REAL NOT NULL,
    change_pct      REAL,
    volume          REAL,
    pre_price       REAL,
    post_price      REAL,
    recorded_at     INTEGER NOT NULL,
    FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_price_history_sub_time
    ON price_history (subscription_id, recorded_at);

INSERT OR IGNORE INTO views (id, name, view_type, is_default) VALUES (1, 'All', 'asset', 1);
INSERT OR IGNORE INTO views (id, name, view_type, is_default) VALUES (2, 'All', 'dex', 1);

INSERT OR IGNORE INTO app_settings (key, value) VALUES ('api_port', '8080');

CREATE TRIGGER IF NOT EXISTS auto_sort_order
AFTER INSERT ON subscriptions
WHEN NEW.sort_order = 0
BEGIN
    UPDATE subscriptions SET sort_order = NEW.id WHERE id = NEW.id;
END;
"#;
