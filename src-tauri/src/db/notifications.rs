use rusqlite::{params, types::Null};

use super::schema::{NotificationChannelRow, NotificationHistoryRow, NotificationRuleRow};
use super::DbPool;

impl DbPool {
    // ── Notification Channels ───────────────────────────────────

    pub fn create_notification_channel(
        &self,
        channel_type: &str,
        name: &str,
        config: &str,
    ) -> Result<i64, String> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO notification_channels (channel_type, name, config, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![channel_type, name, config, now],
        )
        .map_err(|e| format!("Failed to create notification channel: {}", e))?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_notification_channels(&self) -> Result<Vec<NotificationChannelRow>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, channel_type, name, config, created_at FROM notification_channels ORDER BY id")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(NotificationChannelRow {
                    id: row.get(0)?,
                    channel_type: row.get(1)?,
                    name: row.get(2)?,
                    config: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn delete_notification_channel(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM notification_channels WHERE id = ?1", [id])
            .map_err(|e| format!("Failed to delete notification channel: {}", e))?;
        Ok(())
    }

    /// Ensures a default "Local" notification channel exists.
    /// Called once at application startup. Idempotent.
    pub fn ensure_local_channel(&self) -> Result<(), String> {
        let channels = self.list_notification_channels()?;
        let has_local = channels.iter().any(|c| c.channel_type == "local");
        if !has_local {
            self.create_notification_channel("local", "Local", "{}")?;
        }
        Ok(())
    }

    /// Ensures a default "System" notification channel exists.
    /// Called once at application startup. Idempotent.
    pub fn ensure_system_channel(&self) -> Result<(), String> {
        let channels = self.list_notification_channels()?;
        let has_system = channels.iter().any(|c| c.channel_type == "system");
        if !has_system {
            self.create_notification_channel("system", "System", "{}")?;
        }
        Ok(())
    }

    // ── Notification Rules ──────────────────────────────────────

    // 引數對應 notification_rules 資料表欄位，刻意保持平面簽章
    #[allow(clippy::too_many_arguments)]
    pub fn create_notification_rule(
        &self,
        name: &str,
        subscription_id: i64,
        condition_type: &str,
        threshold: f64,
        channel_ids: &str,
        cooldown_secs: i64,
        ai_config: Option<&str>,
    ) -> Result<i64, String> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO notification_rules (name, subscription_id, condition_type, threshold, channel_ids, cooldown_secs, enabled, ai_config, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, ?8)",
            params![name, subscription_id, condition_type, threshold, channel_ids, cooldown_secs, ai_config, now],
        )
        .map_err(|e| format!("Failed to create notification rule: {}", e))?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_notification_rules(&self) -> Result<Vec<NotificationRuleRow>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, subscription_id, condition_type, threshold, channel_ids, cooldown_secs, enabled, ai_config, created_at, updated_at
                 FROM notification_rules ORDER BY id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(NotificationRuleRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    subscription_id: row.get(2)?,
                    condition_type: row.get(3)?,
                    threshold: row.get(4)?,
                    channel_ids: row.get(5)?,
                    cooldown_secs: row.get(6)?,
                    enabled: row.get::<_, i64>(7)? != 0,
                    ai_config: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_notification_rule(&self, id: i64) -> Result<Option<NotificationRuleRow>, String> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT id, name, subscription_id, condition_type, threshold, channel_ids, cooldown_secs, enabled, ai_config, created_at, updated_at
             FROM notification_rules WHERE id = ?1",
            [id],
            |row| {
                Ok(NotificationRuleRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    subscription_id: row.get(2)?,
                    condition_type: row.get(3)?,
                    threshold: row.get(4)?,
                    channel_ids: row.get(5)?,
                    cooldown_secs: row.get(6)?,
                    enabled: row.get::<_, i64>(7)? != 0,
                    ai_config: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        );
        match result {
            Ok(row) => Ok(Some(row)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    // 引數對應 notification_rules 可更新欄位，刻意保持平面簽章
    #[allow(clippy::too_many_arguments)]
    pub fn update_notification_rule(
        &self,
        id: i64,
        name: Option<&str>,
        condition_type: Option<&str>,
        threshold: Option<f64>,
        channel_ids: Option<&str>,
        cooldown_secs: Option<i64>,
        ai_config: Option<Option<&str>>,
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp();
        let mut sets = vec!["updated_at = ?1".to_string()];
        let mut idx = 2u32;

        // Build dynamic SET clause
        if name.is_some() {
            sets.push(format!("name = ?{}", idx));
            idx += 1;
        }
        if condition_type.is_some() {
            sets.push(format!("condition_type = ?{}", idx));
            idx += 1;
        }
        if threshold.is_some() {
            sets.push(format!("threshold = ?{}", idx));
            idx += 1;
        }
        if channel_ids.is_some() {
            sets.push(format!("channel_ids = ?{}", idx));
            idx += 1;
        }
        if cooldown_secs.is_some() {
            sets.push(format!("cooldown_secs = ?{}", idx));
            idx += 1;
        }
        if ai_config.is_some() {
            sets.push(format!("ai_config = ?{}", idx));
            idx += 1;
        }

        let sql = format!(
            "UPDATE notification_rules SET {} WHERE id = ?{}",
            sets.join(", "),
            idx
        );

        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

        // Bind parameters dynamically
        let mut param_idx = 1u32;
        stmt.raw_bind_parameter(param_idx as usize, now)
            .map_err(|e| e.to_string())?;
        param_idx += 1;

        if let Some(v) = name {
            stmt.raw_bind_parameter(param_idx as usize, v)
                .map_err(|e| e.to_string())?;
            param_idx += 1;
        }
        if let Some(v) = condition_type {
            stmt.raw_bind_parameter(param_idx as usize, v)
                .map_err(|e| e.to_string())?;
            param_idx += 1;
        }
        if let Some(v) = threshold {
            stmt.raw_bind_parameter(param_idx as usize, v)
                .map_err(|e| e.to_string())?;
            param_idx += 1;
        }
        if let Some(v) = channel_ids {
            stmt.raw_bind_parameter(param_idx as usize, v)
                .map_err(|e| e.to_string())?;
            param_idx += 1;
        }
        if let Some(v) = cooldown_secs {
            stmt.raw_bind_parameter(param_idx as usize, v)
                .map_err(|e| e.to_string())?;
            param_idx += 1;
        }
        if let Some(v) = ai_config {
            // v is Option<&str>: Some("json") sets the value, None sets it to NULL
            match v {
                Some(json_str) => {
                    stmt.raw_bind_parameter(param_idx as usize, json_str)
                        .map_err(|e| e.to_string())?;
                }
                None => {
                    stmt.raw_bind_parameter(param_idx as usize, Null)
                        .map_err(|e| e.to_string())?;
                }
            }
            param_idx += 1;
        }

        // Bind the WHERE id parameter
        stmt.raw_bind_parameter(param_idx as usize, id)
            .map_err(|e| e.to_string())?;

        stmt.raw_execute()
            .map_err(|e| format!("Failed to update notification rule: {}", e))?;
        Ok(())
    }

    pub fn delete_notification_rule(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM notification_rules WHERE id = ?1", [id])
            .map_err(|e| format!("Failed to delete notification rule: {}", e))?;
        Ok(())
    }

    pub fn toggle_notification_rule(&self, id: i64, enabled: bool) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "UPDATE notification_rules SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
            params![enabled as i64, now, id],
        )
        .map_err(|e| format!("Failed to toggle notification rule: {}", e))?;
        Ok(())
    }

    // ── Notification History ────────────────────────────────────

    pub fn insert_notification_history(
        &self,
        rule_id: i64,
        channel_id: i64,
        status: &str,
        price: f64,
        message: &str,
        error: Option<&str>,
    ) -> Result<i64, String> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO notification_history (rule_id, channel_id, status, price, message, error, sent_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![rule_id, channel_id, status, price, message, error, now],
        )
        .map_err(|e| format!("Failed to write notification history: {}", e))?;
        Ok(conn.last_insert_rowid())
    }

    pub fn query_notification_history(
        &self,
        rule_id: Option<i64>,
        from: Option<i64>,
        to: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<NotificationHistoryRow>, String> {
        let conn = self.conn.lock().unwrap();
        let mut conditions = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut idx = 1u32;

        if let Some(rid) = rule_id {
            conditions.push(format!("rule_id = ?{}", idx));
            param_values.push(Box::new(rid));
            idx += 1;
        }
        if let Some(f) = from {
            conditions.push(format!("sent_at >= ?{}", idx));
            param_values.push(Box::new(f));
            idx += 1;
        }
        if let Some(t) = to {
            conditions.push(format!("sent_at <= ?{}", idx));
            param_values.push(Box::new(t));
            idx += 1;
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let actual_limit = limit.unwrap_or(100);
        let sql = format!(
            "SELECT id, rule_id, channel_id, status, price, message, error, sent_at
             FROM notification_history {} ORDER BY sent_at DESC LIMIT ?{}",
            where_clause, idx
        );

        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

        // Bind all parameters
        for (i, val) in param_values.iter().enumerate() {
            stmt.raw_bind_parameter(i + 1, val.as_ref())
                .map_err(|e| e.to_string())?;
        }
        stmt.raw_bind_parameter(idx as usize, actual_limit)
            .map_err(|e| e.to_string())?;

        let mut result = Vec::new();
        let mut rows = stmt.raw_query();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            result.push(NotificationHistoryRow {
                id: row.get(0).map_err(|e| e.to_string())?,
                rule_id: row.get(1).map_err(|e| e.to_string())?,
                channel_id: row.get(2).map_err(|e| e.to_string())?,
                status: row.get(3).map_err(|e| e.to_string())?,
                price: row.get(4).map_err(|e| e.to_string())?,
                message: row.get(5).map_err(|e| e.to_string())?,
                error: row.get(6).map_err(|e| e.to_string())?,
                sent_at: row.get(7).map_err(|e| e.to_string())?,
            });
        }
        Ok(result)
    }
}
