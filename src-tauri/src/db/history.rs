use chrono::Timelike;
use rusqlite::params;

use super::schema::{PriceHistoryRow, PriceRecord, HistoryStats};
use super::DbPool;

impl DbPool {
    // ── Price History ───────────────────────────────────────────

    pub fn write_price_history(
        &self,
        provider_id: &str,
        data: &[PriceRecord],
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
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
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
            .execute(
                "DELETE FROM price_history WHERE recorded_at < ?1",
                [before_ts],
            )
            .map_err(|e| e.to_string())?;
        Ok(deleted as i64)
    }

    pub fn purge_all_history(&self) -> Result<i64, String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM price_history", [])
            .map_err(|e| e.to_string())?;
        Ok(conn.changes() as i64)
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

    /// Test helper: insert price history records directly (bypasses record_enabled and dedup checks)
    #[cfg(test)]
    pub fn insert_price_history_for_test(
        &self,
        subscription_id: i64,
        provider_id: &str,
        records: &[(f64, Option<f64>, Option<f64>, i64)], // (price, change_pct, volume, recorded_at)
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        for (price, change_pct, volume, recorded_at) in records {
            conn.execute(
                "INSERT INTO price_history (subscription_id, provider_id, price, change_pct, volume, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![subscription_id, provider_id, price, change_pct, volume, recorded_at],
            )
            .map_err(|e| format!("Failed to insert test price history: {}", e))?;
        }
        Ok(())
    }
}
