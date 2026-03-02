/// StockenBoard 統一 DB 存取層
///
/// 所有 SQLite 操作集中在此模組，前端不再直接操作 SQL。
/// 使用 `Mutex<Connection>` 確保寫入操作序列化，搭配 WAL mode 允許並行讀取。
use chrono::Timelike;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
"#;

// ── Data types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: i64,
    pub sub_type: String,
    pub symbol: String,
    pub display_name: Option<String>,
    pub selected_provider_id: String,
    pub asset_type: String,
    pub pool_address: Option<String>,
    pub token_from_address: Option<String>,
    pub token_to_address: Option<String>,
    pub sort_order: i64,
    pub record_enabled: i64,
    pub record_from_hour: Option<i64>,
    pub record_to_hour: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSettingsRow {
    pub provider_id: String,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub api_url: Option<String>,
    pub refresh_interval: Option<i64>,
    pub connection_type: String,
    pub record_from_hour: Option<i64>,
    pub record_to_hour: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewRow {
    pub id: i64,
    pub name: String,
    pub view_type: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewSubCount {
    pub view_id: i64,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceHistoryRow {
    pub id: i64,
    pub subscription_id: i64,
    pub provider_id: String,
    pub price: f64,
    pub change_pct: Option<f64>,
    pub volume: Option<f64>,
    pub pre_price: Option<f64>,
    pub post_price: Option<f64>,
    pub recorded_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryStats {
    pub total: i64,
    pub oldest: Option<i64>,
    pub newest: Option<i64>,
}

// ── Export/Import types ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub subscriptions: Vec<ExportSubscription>,
    pub views: Vec<ExportView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSubscription {
    pub symbol: String,
    pub display_name: Option<String>,
    pub selected_provider_id: String,
    pub asset_type: String,
    pub sub_type: String,
    pub pool_address: Option<String>,
    pub token_from_address: Option<String>,
    pub token_to_address: Option<String>,
    pub record_enabled: Option<bool>,
    pub record_from_hour: Option<i64>,
    pub record_to_hour: Option<i64>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportView {
    pub name: String,
    pub view_type: String,
    pub symbols: Vec<String>,
}

// ── DbPool ──────────────────────────────────────────────────────

pub struct DbPool {
    conn: Mutex<Connection>,
}

impl DbPool {
    pub fn open(path: &PathBuf) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| format!("開啟 DB 失敗: {}", e))?;
        // 啟用 WAL mode + busy timeout，應對高併發
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;
             PRAGMA foreign_keys=ON;",
        )
        .map_err(|e| format!("設定 PRAGMA 失敗: {}", e))?;
        // 初始化 schema
        conn.execute_batch(SCHEMA)
            .map_err(|e| format!("初始化 schema 失敗: {}", e))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    // ── App Settings ────────────────────────────────────────────

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .ok()
        .map_or(Ok(None), |v| Ok(Some(v)))
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )
        .map_err(|e| format!("設定 app_settings 失敗: {}", e))?;
        Ok(())
    }

    pub fn reset_all_data(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        // 刪除所有資料
        conn.execute_batch(
            "DELETE FROM price_history;
             DELETE FROM view_subscriptions;
             DELETE FROM subscriptions;
             DELETE FROM views;
             DELETE FROM provider_settings;
             DELETE FROM app_settings;"
        ).map_err(|e| format!("刪除所有資料失敗: {}", e))?;

        // 重新插入預設 Views
        conn.execute_batch(
            "INSERT OR IGNORE INTO views (id, name, view_type, is_default) VALUES (1, 'All', 'asset', 1);
             INSERT OR IGNORE INTO views (id, name, view_type, is_default) VALUES (2, 'All', 'dex', 1);
             INSERT OR IGNORE INTO app_settings (key, value) VALUES ('api_port', '8080');
             INSERT OR IGNORE INTO app_settings (key, value) VALUES ('api_enabled', '0');"
        ).map_err(|e| format!("還原預設資料失敗: {}", e))?;

        Ok(())
    }

    // ── Subscriptions ───────────────────────────────────────────

    pub fn list_subscriptions(&self, sub_type: &str) -> Result<Vec<Subscription>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, sub_type, symbol, display_name, selected_provider_id, asset_type,
                    pool_address, token_from_address, token_to_address, sort_order,
                    record_enabled, record_from_hour, record_to_hour
                 FROM subscriptions WHERE sub_type = ?1 ORDER BY sort_order, id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([sub_type], |row| {
                Ok(Subscription {
                    id: row.get(0)?,
                    sub_type: row.get(1)?,
                    symbol: row.get(2)?,
                    display_name: row.get(3)?,
                    selected_provider_id: row.get(4)?,
                    asset_type: row.get(5)?,
                    pool_address: row.get(6)?,
                    token_from_address: row.get(7)?,
                    token_to_address: row.get(8)?,
                    sort_order: row.get(9)?,
                    record_enabled: row.get(10)?,
                    record_from_hour: row.get(11)?,
                    record_to_hour: row.get(12)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn list_all_subscriptions(&self) -> Result<Vec<Subscription>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, sub_type, symbol, display_name, selected_provider_id, asset_type,
                    pool_address, token_from_address, token_to_address, sort_order,
                    record_enabled, record_from_hour, record_to_hour
                 FROM subscriptions ORDER BY sort_order, id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Subscription {
                    id: row.get(0)?,
                    sub_type: row.get(1)?,
                    symbol: row.get(2)?,
                    display_name: row.get(3)?,
                    selected_provider_id: row.get(4)?,
                    asset_type: row.get(5)?,
                    pool_address: row.get(6)?,
                    token_from_address: row.get(7)?,
                    token_to_address: row.get(8)?,
                    sort_order: row.get(9)?,
                    record_enabled: row.get(10)?,
                    record_from_hour: row.get(11)?,
                    record_to_hour: row.get(12)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// 新增訂閱，回傳新 ID。使用 INSERT OR IGNORE 避免重複。
    pub fn add_subscription(
        &self,
        sub_type: &str,
        symbol: &str,
        display_name: Option<&str>,
        provider_id: &str,
        asset_type: &str,
        pool_address: Option<&str>,
        token_from: Option<&str>,
        token_to: Option<&str>,
    ) -> Result<i64, String> {
        let conn = self.conn.lock().unwrap();
        let changed = conn
            .execute(
                "INSERT OR IGNORE INTO subscriptions (sub_type, symbol, display_name, selected_provider_id, asset_type, pool_address, token_from_address, token_to_address)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![sub_type, symbol, display_name, provider_id, asset_type, pool_address, token_from, token_to],
            )
            .map_err(|e| format!("新增訂閱失敗: {}", e))?;
        if changed == 0 {
            return Err("訂閱已存在".to_string());
        }
        Ok(conn.last_insert_rowid())
    }

    pub fn update_subscription(
        &self,
        id: i64,
        symbol: &str,
        display_name: Option<&str>,
        provider_id: &str,
        asset_type: &str,
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE subscriptions SET symbol = ?1, display_name = ?2, selected_provider_id = ?3, asset_type = ?4 WHERE id = ?5",
            params![symbol, display_name, provider_id, asset_type, id],
        )
        .map_err(|e| format!("更新訂閱失敗: {}", e))?;
        Ok(())
    }

    pub fn remove_subscription(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM subscriptions WHERE id = ?1", [id])
            .map_err(|e| format!("刪除訂閱失敗: {}", e))?;
        Ok(())
    }

    pub fn remove_subscriptions(&self, ids: &[i64]) -> Result<(), String> {
        if ids.is_empty() {
            return Ok(());
        }
        let conn = self.conn.lock().unwrap();
        let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
        let sql = format!("DELETE FROM subscriptions WHERE id IN ({})", placeholders.join(","));
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        stmt.execute(params_refs.as_slice())
            .map_err(|e| format!("批量刪除失敗: {}", e))?;
        Ok(())
    }

    pub fn toggle_record(&self, id: i64, enabled: bool) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE subscriptions SET record_enabled = ?1 WHERE id = ?2",
            params![enabled as i64, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_record_hours(&self, id: i64, from: Option<i64>, to: Option<i64>) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE subscriptions SET record_from_hour = ?1, record_to_hour = ?2 WHERE id = ?3",
            params![from, to, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Provider Settings ───────────────────────────────────────

    pub fn list_provider_settings(&self) -> Result<Vec<ProviderSettingsRow>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT provider_id, api_key, api_secret, api_url, refresh_interval, connection_type, record_from_hour, record_to_hour FROM provider_settings")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ProviderSettingsRow {
                    provider_id: row.get(0)?,
                    api_key: row.get(1)?,
                    api_secret: row.get(2)?,
                    api_url: row.get(3)?,
                    refresh_interval: row.get(4)?,
                    connection_type: row.get(5)?,
                    record_from_hour: row.get(6)?,
                    record_to_hour: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_provider_settings(&self, provider_id: &str) -> Result<Option<ProviderSettingsRow>, String> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT provider_id, api_key, api_secret, api_url, refresh_interval, connection_type, record_from_hour, record_to_hour FROM provider_settings WHERE provider_id = ?1",
            [provider_id],
            |row| {
                Ok(ProviderSettingsRow {
                    provider_id: row.get(0)?,
                    api_key: row.get(1)?,
                    api_secret: row.get(2)?,
                    api_url: row.get(3)?,
                    refresh_interval: row.get(4)?,
                    connection_type: row.get(5)?,
                    record_from_hour: row.get(6)?,
                    record_to_hour: row.get(7)?,
                })
            },
        );
        match result {
            Ok(row) => Ok(Some(row)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn upsert_provider_settings(
        &self,
        provider_id: &str,
        api_key: Option<&str>,
        api_secret: Option<&str>,
        api_url: Option<&str>,
        refresh_interval: Option<i64>,
        connection_type: &str,
        record_from_hour: Option<i64>,
        record_to_hour: Option<i64>,
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO provider_settings (provider_id, api_key, api_secret, api_url, refresh_interval, connection_type, record_from_hour, record_to_hour)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(provider_id) DO UPDATE SET
               api_key = ?2, api_secret = ?3, api_url = ?4, refresh_interval = ?5, connection_type = ?6, record_from_hour = ?7, record_to_hour = ?8",
            params![provider_id, api_key, api_secret, api_url, refresh_interval, connection_type, record_from_hour, record_to_hour],
        )
        .map_err(|e| format!("更新 provider 設定失敗: {}", e))?;
        Ok(())
    }

    pub fn set_provider_record_hours(
        &self,
        provider_id: &str,
        from: Option<i64>,
        to: Option<i64>,
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO provider_settings (provider_id, record_from_hour, record_to_hour, connection_type)
             VALUES (?1, ?2, ?3, 'rest')
             ON CONFLICT(provider_id) DO UPDATE SET record_from_hour = ?2, record_to_hour = ?3",
            params![provider_id, from, to],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn has_api_key(&self, provider_id: &str) -> bool {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT api_key FROM provider_settings WHERE provider_id = ?1",
            [provider_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .map(|k| !k.is_empty())
        .unwrap_or(false)
    }

    // ── Views ───────────────────────────────────────────────────

    pub fn list_views(&self, view_type: &str) -> Result<Vec<ViewRow>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, view_type, is_default FROM views WHERE view_type = ?1 ORDER BY id")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([view_type], |row| {
                Ok(ViewRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    view_type: row.get(2)?,
                    is_default: row.get::<_, i64>(3)? != 0,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn create_view(&self, name: &str, view_type: &str) -> Result<i64, String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO views (name, view_type, is_default) VALUES (?1, ?2, 0)",
            params![name, view_type],
        )
        .map_err(|e| format!("建立 view 失敗: {}", e))?;
        Ok(conn.last_insert_rowid())
    }

    pub fn rename_view(&self, id: i64, name: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE views SET name = ?1 WHERE id = ?2", params![name, id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_view(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM views WHERE id = ?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_view_sub_counts(&self) -> Result<Vec<ViewSubCount>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT view_id, COUNT(*) as cnt FROM view_subscriptions GROUP BY view_id")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ViewSubCount {
                    view_id: row.get(0)?,
                    count: row.get(1)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_view_subscription_ids(&self, view_id: i64) -> Result<Vec<i64>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT subscription_id FROM view_subscriptions WHERE view_id = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([view_id], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn add_sub_to_view(&self, view_id: i64, subscription_id: i64) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO view_subscriptions (view_id, subscription_id) VALUES (?1, ?2)",
            params![view_id, subscription_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn remove_sub_from_view(&self, view_id: i64, subscription_id: i64) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM view_subscriptions WHERE view_id = ?1 AND subscription_id = ?2",
            params![view_id, subscription_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Price History ───────────────────────────────────────────

    pub fn write_price_history(
        &self,
        provider_id: &str,
        data: &[(String, f64, Option<f64>, Option<f64>, Option<f64>, Option<f64>)],
    ) {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp();
        let local_hour = chrono::Local::now().hour();

        for (symbol, price, change_pct, volume, pre_price, post_price) in data {
            // 找到訂閱 ID 和紀錄設定
            let sub_row: Option<(i64, Option<i64>, Option<i64>)> = conn
                .prepare_cached("SELECT id, record_from_hour, record_to_hour FROM subscriptions WHERE symbol = ?1 AND selected_provider_id = ?2")
                .ok()
                .and_then(|mut stmt| {
                    stmt.query_row(params![symbol, provider_id], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                    })
                    .ok()
                });
            let (sub_id, sub_from, sub_to) = match sub_row {
                Some(r) => r,
                None => continue,
            };

            // 檢查是否啟用紀錄
            let record_enabled: bool = conn
                .query_row(
                    "SELECT record_enabled FROM subscriptions WHERE id = ?1",
                    [sub_id],
                    |row| row.get::<_, i64>(0),
                )
                .map(|v| v != 0)
                .unwrap_or(false);
            if !record_enabled {
                continue;
            }

            // 紀錄時段檢查
            let (from_h, to_h) = if let (Some(from), Some(to)) = (sub_from, sub_to) {
                (from as u32, to as u32)
            } else {
                let prov_hours: Option<(Option<i64>, Option<i64>)> = conn
                    .prepare_cached("SELECT record_from_hour, record_to_hour FROM provider_settings WHERE provider_id = ?1")
                    .ok()
                    .and_then(|mut stmt| {
                        stmt.query_row([provider_id], |row| Ok((row.get(0)?, row.get(1)?)))
                            .ok()
                    });
                match prov_hours {
                    Some((Some(pf), Some(pt))) => (pf as u32, pt as u32),
                    _ => (0, 24),
                }
            };

            if from_h != 0 || to_h != 24 {
                let in_window = if from_h <= to_h {
                    local_hour >= from_h && local_hour < to_h
                } else {
                    local_hour >= from_h || local_hour < to_h
                };
                if !in_window {
                    continue;
                }
            }

            // 5 秒去重
            let recent: bool = conn
                .prepare_cached("SELECT 1 FROM price_history WHERE subscription_id = ?1 AND recorded_at > ?2 LIMIT 1")
                .ok()
                .and_then(|mut stmt| {
                    stmt.query_row(params![sub_id, now - 5], |_| Ok(true)).ok()
                })
                .unwrap_or(false);
            if recent {
                continue;
            }

            let _ = conn.execute(
                "INSERT INTO price_history (subscription_id, provider_id, price, change_pct, volume, pre_price, post_price, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![sub_id, provider_id, price, change_pct, volume, pre_price, post_price, now],
            );
        }
    }

    pub fn get_price_history(
        &self,
        subscription_id: i64,
        from: Option<i64>,
        to: Option<i64>,
        limit: i64,
    ) -> Result<Vec<PriceHistoryRow>, String> {
        let conn = self.conn.lock().unwrap();
        let mut sql = "SELECT id, subscription_id, provider_id, price, change_pct, volume, pre_price, post_price, recorded_at FROM price_history WHERE subscription_id = ?1".to_string();
        let mut p: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(subscription_id)];
        if let Some(f) = from {
            p.push(Box::new(f));
            sql.push_str(&format!(" AND recorded_at >= ?{}", p.len()));
        }
        if let Some(t) = to {
            p.push(Box::new(t));
            sql.push_str(&format!(" AND recorded_at <= ?{}", p.len()));
        }
        sql.push_str(" ORDER BY recorded_at DESC");
        p.push(Box::new(limit));
        sql.push_str(&format!(" LIMIT ?{}", p.len()));

        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = p.iter().map(|b| b.as_ref()).collect();
        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(PriceHistoryRow {
                    id: row.get(0)?,
                    subscription_id: row.get(1)?,
                    provider_id: row.get(2)?,
                    price: row.get(3)?,
                    change_pct: row.get(4)?,
                    volume: row.get(5)?,
                    pre_price: row.get(6)?,
                    post_price: row.get(7)?,
                    recorded_at: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_history_stats(&self, subscription_id: i64) -> Result<HistoryStats, String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*), MIN(recorded_at), MAX(recorded_at) FROM price_history WHERE subscription_id = ?1",
            [subscription_id],
            |row| {
                Ok(HistoryStats {
                    total: row.get(0)?,
                    oldest: row.get(1)?,
                    newest: row.get(2)?,
                })
            },
        )
        .map_err(|e| e.to_string())
    }

    pub fn cleanup_history(&self, before_ts: i64) -> Result<i64, String> {
        let conn = self.conn.lock().unwrap();
        let deleted = conn
            .execute("DELETE FROM price_history WHERE recorded_at < ?1", [before_ts])
            .map_err(|e| e.to_string())?;
        Ok(deleted as i64)
    }

    pub fn purge_all_history(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM price_history", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_history_for_subscription(&self, subscription_id: i64) -> Result<i64, String> {
        let conn = self.conn.lock().unwrap();
        let deleted = conn
            .execute(
                "DELETE FROM price_history WHERE subscription_id = ?1",
                [subscription_id],
            )
            .map_err(|e| e.to_string())?;
        Ok(deleted as i64)
    }

    // ── Export / Import ─────────────────────────────────────────

    pub fn export_data(&self) -> Result<ExportData, String> {
        let conn = self.conn.lock().unwrap();

        // Export views
        let mut views_out = Vec::new();
        {
            let mut stmt = conn
                .prepare("SELECT id, name, view_type, is_default FROM views ORDER BY id")
                .map_err(|e| e.to_string())?;
            let view_rows: Vec<(i64, String, String, i64)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();

            for (view_id, name, view_type, is_default) in &view_rows {
                if *is_default != 0 {
                    continue;
                }
                let mut sub_stmt = conn
                    .prepare("SELECT s.symbol FROM view_subscriptions vs JOIN subscriptions s ON s.id = vs.subscription_id WHERE vs.view_id = ?1")
                    .map_err(|e| e.to_string())?;
                let symbols: Vec<String> = sub_stmt
                    .query_map([view_id], |row| row.get(0))
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect();
                views_out.push(ExportView {
                    name: name.clone(),
                    view_type: view_type.clone(),
                    symbols,
                });
            }
        }

        // Export subscriptions
        let subs_out: Vec<ExportSubscription>;
        {
            let mut stmt = conn
                .prepare("SELECT symbol, display_name, selected_provider_id, asset_type, sub_type, pool_address, token_from_address, token_to_address, record_enabled, record_from_hour, record_to_hour, sort_order FROM subscriptions ORDER BY sort_order, id")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(ExportSubscription {
                        symbol: row.get(0)?,
                        display_name: row.get(1)?,
                        selected_provider_id: row.get(2)?,
                        asset_type: row.get(3)?,
                        sub_type: row.get(4)?,
                        pool_address: row.get(5)?,
                        token_from_address: row.get(6)?,
                        token_to_address: row.get(7)?,
                        record_enabled: row.get(8)?,
                        record_from_hour: row.get(9)?,
                        record_to_hour: row.get(10)?,
                        sort_order: row.get(11)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            subs_out = rows.filter_map(|r| r.ok()).collect();
        }

        Ok(ExportData {
            subscriptions: subs_out,
            views: views_out,
        })
    }

    pub fn import_data(&self, data: &ExportData) -> Result<(usize, usize), String> {
        let conn = self.conn.lock().unwrap();
        let mut imported = 0usize;
        let mut skipped = 0usize;

        // Import subscriptions
        for sub in &data.subscriptions {
            let changed = conn
                .execute(
                    "INSERT OR IGNORE INTO subscriptions (sub_type, symbol, display_name, selected_provider_id, asset_type, pool_address, token_from_address, token_to_address, record_enabled, record_from_hour, record_to_hour, sort_order)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        sub.sub_type, sub.symbol, sub.display_name, sub.selected_provider_id,
                        sub.asset_type, sub.pool_address, sub.token_from_address, sub.token_to_address,
                        sub.record_enabled.unwrap_or(false), sub.record_from_hour, sub.record_to_hour, sub.sort_order.unwrap_or(0)
                    ],
                )
                .unwrap_or(0);
            if changed > 0 {
                imported += 1;
            } else {
                skipped += 1;
            }
        }

        // Import views
        for view in &data.views {
            let view_id = conn
                .execute(
                    "INSERT OR IGNORE INTO views (name, view_type, is_default) VALUES (?1, ?2, 0)",
                    params![view.name, view.view_type],
                )
                .ok()
                .map(|_| conn.last_insert_rowid());

            if let Some(vid) = view_id {
                // 如果是新建的 view，插入其 symbols 的關聯
                if vid > 0 {
                    for sym in &view.symbols {
                        let sub_id: Option<i64> = conn
                            .query_row(
                                "SELECT id FROM subscriptions WHERE symbol = ?1",
                                [sym],
                                |row| row.get(0),
                            )
                            .ok();
                        if let Some(sid) = sub_id {
                            let _ = conn.execute(
                                "INSERT OR IGNORE INTO view_subscriptions (view_id, subscription_id) VALUES (?1, ?2)",
                                params![vid, sid],
                            );
                        }
                    }
                }
            }
        }

        Ok((imported, skipped))
    }

    // ── Polling 專用 ────────────────────────────────────────────

    /// 為 Polling 讀取所有訂閱（可選 visible_ids 過濾）
    pub fn read_polling_subscriptions(
        &self,
        visible_ids: Option<&HashSet<i64>>,
    ) -> Result<Vec<(i64, String, String, bool)>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, sub_type, symbol, selected_provider_id, pool_address, token_from_address, token_to_address, record_enabled FROM subscriptions")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let id: i64 = row.get(0)?;
                let sub_type: String = row.get(1)?;
                let symbol: String = row.get(2)?;
                let provider_id: String = row.get(3)?;
                let pool_address: Option<String> = row.get(4)?;
                let token_from: Option<String> = row.get(5)?;
                let token_to: Option<String> = row.get(6)?;
                let record_enabled: i64 = row.get(7)?;

                let final_symbol = if sub_type == "dex" {
                    format!(
                        "{}:{}:{}",
                        pool_address.unwrap_or_default(),
                        token_from.unwrap_or_default(),
                        token_to.unwrap_or_default()
                    )
                } else {
                    symbol
                };

                Ok((id, final_symbol, provider_id, record_enabled != 0))
            })
            .map_err(|e| e.to_string())?;
        let all: Vec<(i64, String, String, bool)> = rows.filter_map(|r| r.ok()).collect();
        Ok(match visible_ids {
            Some(ids) => all.into_iter().filter(|(id, _, _, _)| ids.contains(id)).collect(),
            None => all,
        })
    }

    /// 為 Polling 讀取所有 provider 設定
    pub fn read_polling_provider_settings(
        &self,
    ) -> Result<
        std::collections::HashMap<String, (Option<String>, Option<String>, Option<String>, Option<i64>)>,
        String,
    > {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT provider_id, api_key, api_secret, api_url, refresh_interval FROM provider_settings")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    (row.get(1)?, row.get(2)?, row.get(3).ok().flatten(), row.get(4)?),
                ))
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
