/// StockenBoard 統一 DB 存取層
///
/// 所有 SQLite 操作集中在此模組，前端不再直接操作 SQL。
/// 使用 `Mutex<Connection>` 確保寫入操作序列化，搭配 WAL mode 允許並行讀取。
mod history;
mod notifications;
mod providers;
mod schema;
mod settings;
mod subscriptions;
mod views;

pub use schema::*;

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

// ── Schema ──────────────────────────────────────────────────────

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
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('api_enabled', '0');

CREATE TRIGGER IF NOT EXISTS auto_sort_order
AFTER INSERT ON subscriptions
WHEN NEW.sort_order = 0
BEGIN
    UPDATE subscriptions SET sort_order = NEW.id WHERE id = NEW.id;
END;

-- 通知通道設定
CREATE TABLE IF NOT EXISTS notification_channels (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    channel_type TEXT NOT NULL,
    name         TEXT NOT NULL,
    config       TEXT NOT NULL,
    created_at   INTEGER NOT NULL
);

-- 通知規則
CREATE TABLE IF NOT EXISTS notification_rules (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL,
    subscription_id INTEGER NOT NULL,
    condition_type  TEXT NOT NULL,
    threshold       REAL NOT NULL,
    channel_ids     TEXT NOT NULL,
    cooldown_secs   INTEGER NOT NULL DEFAULT 300,
    enabled         INTEGER NOT NULL DEFAULT 1,
    ai_config       TEXT,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
);

-- 通知歷史
CREATE TABLE IF NOT EXISTS notification_history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_id     INTEGER NOT NULL,
    channel_id  INTEGER NOT NULL,
    status      TEXT NOT NULL,
    price       REAL NOT NULL,
    message     TEXT NOT NULL,
    error       TEXT,
    sent_at     INTEGER NOT NULL,
    FOREIGN KEY (rule_id) REFERENCES notification_rules(id) ON DELETE CASCADE,
    FOREIGN KEY (channel_id) REFERENCES notification_channels(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_notification_history_rule_time
    ON notification_history (rule_id, sent_at);

CREATE INDEX IF NOT EXISTS idx_notification_rules_sub
    ON notification_rules (subscription_id);
"#;

// ── DbPool ──────────────────────────────────────────────────────

pub struct DbPool {
    pub(crate) conn: Mutex<Connection>,
}

impl DbPool {
    pub fn open(path: &PathBuf) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| format!("Failed to open DB: {}", e))?;
        // 啟用 WAL mode + busy timeout，應對高併發
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;
             PRAGMA foreign_keys=ON;",
        )
        .map_err(|e| format!("Failed to set PRAGMA: {}", e))?;
        // 初始化 schema
        conn.execute_batch(SCHEMA)
            .map_err(|e| format!("Failed to initialize schema: {}", e))?;

        // ── Migrations（為既有資料庫新增欄位）──────────────────────
        // ALTER TABLE 無 IF NOT EXISTS，忽略 "duplicate column" 錯誤即可
        let _ = conn.execute_batch("ALTER TABLE notification_rules ADD COLUMN ai_config TEXT;");
        let _ = conn.execute_batch(
            "ALTER TABLE notification_rules ADD COLUMN subscription_ids TEXT;",
        );

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: open an in-memory DB (schema initialized by `DbPool::open`).
    fn open_test_db() -> DbPool {
        DbPool::open(&PathBuf::from(":memory:")).unwrap()
    }

    /// Helper: insert an `asset` subscription, returning its id.
    fn add_asset(db: &DbPool, symbol: &str, provider: &str) -> i64 {
        db.add_subscription("asset", symbol, None, provider, "crypto", None, None, None)
            .unwrap()
    }

    /// Helper: insert a `dex` subscription, returning its id.
    fn add_dex(db: &DbPool, symbol: &str, provider: &str) -> i64 {
        db.add_subscription(
            "dex",
            symbol,
            None,
            provider,
            "dex",
            Some("0xpool"),
            Some("0xfrom"),
            Some("0xto"),
        )
        .unwrap()
    }

    /// `list_all_subscriptions` returns both `asset` and `dex` rows, and the
    /// total count equals (asset count + dex count). Mirrors how
    /// `engine.rs::load_rules_from_db` consumes the same method.
    ///
    /// Validates: Requirements 1.2, 2.1, 2.2
    #[test]
    fn list_all_subscriptions_returns_mixed_types_with_full_count() {
        let db = open_test_db();

        // Fixture: 2 asset + 2 dex (mixed providers to keep UNIQUE constraint happy).
        add_asset(&db, "BTC/USDT", "binance");
        add_asset(&db, "ETH/USDT", "binance");
        add_dex(&db, "WETH/USDC", "uniswap");
        add_dex(&db, "PEPE/WETH", "uniswap");

        let asset_count = db.list_subscriptions("asset").unwrap().len();
        let dex_count = db.list_subscriptions("dex").unwrap().len();
        let all = db.list_all_subscriptions().unwrap();

        // Count contract: total == asset count + dex count.
        assert_eq!(asset_count, 2);
        assert_eq!(dex_count, 2);
        assert_eq!(all.len(), asset_count + dex_count);

        // Both sub_types are present in the combined result.
        let has_asset = all.iter().any(|s| s.sub_type == "asset");
        let has_dex = all.iter().any(|s| s.sub_type == "dex");
        assert!(has_asset, "expected at least one asset subscription");
        assert!(has_dex, "expected at least one dex subscription");
    }

    /// Asset-only data set: result contains only `asset` and count matches.
    ///
    /// Validates: Requirements 2.1, 2.2
    #[test]
    fn list_all_subscriptions_asset_only() {
        let db = open_test_db();
        add_asset(&db, "BTC/USDT", "binance");
        add_asset(&db, "SOL/USDT", "binance");

        let all = db.list_all_subscriptions().unwrap();
        let asset_count = db.list_subscriptions("asset").unwrap().len();
        let dex_count = db.list_subscriptions("dex").unwrap().len();

        assert_eq!(dex_count, 0);
        assert_eq!(all.len(), asset_count + dex_count);
        assert!(all.iter().all(|s| s.sub_type == "asset"));
    }

    /// Empty DB: `list_all_subscriptions` returns an empty vec (count == 0).
    ///
    /// Validates: Requirements 2.1, 2.2
    #[test]
    fn list_all_subscriptions_empty() {
        let db = open_test_db();
        let all = db.list_all_subscriptions().unwrap();
        assert_eq!(all.len(), 0);
    }

    /// `reset_all_data` clears notification_channels, notification_rules, and
    /// notification_history tables along with all other data.
    #[test]
    fn reset_all_data_clears_notification_tables() {
        let db = open_test_db();

        // Insert a subscription (needed as FK target for notification_rules)
        let sub_id = add_asset(&db, "BTC/USDT", "binance");

        // Insert a notification channel
        let channel_id = db
            .create_notification_channel("telegram", "Test Channel", r#"{"bot_token":"x","chat_id":"1"}"#)
            .unwrap();

        // Insert a notification rule referencing the subscription and channel
        let rule_id = db
            .create_notification_rule(
                "Price Alert",
                sub_id,
                "above",
                50000.0,
                &channel_id.to_string(),
                300,
                None,
                None,
            )
            .unwrap();

        // Insert a notification history entry
        db.insert_notification_history(rule_id, channel_id, "sent", 51000.0, "Price above 50k", None)
            .unwrap();

        // Verify data exists before reset
        assert!(!db.list_notification_channels().unwrap().is_empty());
        assert!(!db.list_notification_rules().unwrap().is_empty());
        assert!(!db.query_notification_history(None, None, None, None).unwrap().is_empty());

        // Perform reset
        db.reset_all_data().unwrap();

        // All notification tables should be empty
        assert!(db.list_notification_channels().unwrap().is_empty(), "notification_channels should be empty after reset");
        assert!(db.list_notification_rules().unwrap().is_empty(), "notification_rules should be empty after reset");
        assert!(db.query_notification_history(None, None, None, None).unwrap().is_empty(), "notification_history should be empty after reset");
    }
}
