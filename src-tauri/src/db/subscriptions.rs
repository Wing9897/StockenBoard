use rusqlite::params;
use std::collections::HashSet;

use super::schema::Subscription;
use super::DbPool;

impl DbPool {
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
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
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
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    /// 新增訂閱，回傳新 ID。使用 INSERT OR IGNORE 避免重複。
    // 引數對應 subscriptions 資料表欄位，刻意保持平面簽章
    #[allow(clippy::too_many_arguments)]
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
            .map_err(|e| format!("Failed to add subscription: {}", e))?;
        if changed == 0 {
            return Err("Subscription already exists".to_string());
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
        .map_err(|e| format!("Failed to update subscription: {}", e))?;
        Ok(())
    }

    pub fn remove_subscription(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM subscriptions WHERE id = ?1", [id])
            .map_err(|e| format!("Failed to delete subscription: {}", e))?;
        Ok(())
    }

    pub fn remove_subscriptions(&self, ids: &[i64]) -> Result<(), String> {
        if ids.is_empty() {
            return Ok(());
        }
        let conn = self.conn.lock().unwrap();
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "DELETE FROM subscriptions WHERE id IN ({})",
            placeholders.join(",")
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        stmt.execute(params_refs.as_slice())
            .map_err(|e| format!("Batch delete failed: {}", e))?;
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

    pub fn set_record_hours(
        &self,
        id: i64,
        from: Option<i64>,
        to: Option<i64>,
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE subscriptions SET record_from_hour = ?1, record_to_hour = ?2 WHERE id = ?3",
            params![from, to, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
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
            Some(ids) => all
                .into_iter()
                .filter(|(id, _, _, _)| ids.contains(id))
                .collect(),
            None => all,
        })
    }
}
