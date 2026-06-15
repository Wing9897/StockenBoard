use rusqlite::params;

use super::schema::{ExportData, ExportSubscription, ExportView};
use super::DbPool;

impl DbPool {
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
        .map_err(|e| format!("Failed to set app_settings: {}", e))?;
        Ok(())
    }

    /// 儲存 AI Provider Config 至 settings 表
    ///
    /// 將 base_url、model 存為明文，api_key 加密後儲存。
    /// 若 api_key 為 None 或空字串，則儲存空字串。
    pub fn save_ai_provider_config(
        &self,
        base_url: &str,
        model: &str,
        api_key: Option<&str>,
    ) -> Result<(), String> {
        // 驗證必要欄位
        if base_url.trim().is_empty() {
            return Err("base_url must not be empty".to_string());
        }
        if model.trim().is_empty() {
            return Err("model must not be empty".to_string());
        }

        // 加密 api_key（若有提供且非空）
        let encrypted_key = match api_key {
            Some(key) if !key.is_empty() => crate::notifications::crypto::encrypt_token(key)?,
            _ => String::new(),
        };

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES ('ai_base_url', ?1) ON CONFLICT(key) DO UPDATE SET value = ?1",
            params![base_url],
        )
        .map_err(|e| format!("Failed to save ai_base_url: {}", e))?;

        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES ('ai_model', ?1) ON CONFLICT(key) DO UPDATE SET value = ?1",
            params![model],
        )
        .map_err(|e| format!("Failed to save ai_model: {}", e))?;

        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES ('ai_api_key', ?1) ON CONFLICT(key) DO UPDATE SET value = ?1",
            params![encrypted_key],
        )
        .map_err(|e| format!("Failed to save ai_api_key: {}", e))?;

        Ok(())
    }

    /// 從 settings 表載入 AI Provider Config
    ///
    /// 若 base_url 或 model 未設定，回傳 None。
    /// api_key 會自動解密；若為空字串則回傳 None。
    pub fn load_ai_provider_config(
        &self,
    ) -> Result<Option<crate::notifications::models::AiProviderConfig>, String> {
        let conn = self.conn.lock().unwrap();

        let base_url: Option<String> = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'ai_base_url'",
                [],
                |row| row.get(0),
            )
            .ok();

        let model: Option<String> = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'ai_model'",
                [],
                |row| row.get(0),
            )
            .ok();

        let encrypted_key: Option<String> = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'ai_api_key'",
                [],
                |row| row.get(0),
            )
            .ok();

        // 若 base_url 或 model 未設定，視為尚未設定
        let base_url = match base_url {
            Some(ref u) if !u.is_empty() => u.clone(),
            _ => return Ok(None),
        };
        let model = match model {
            Some(ref m) if !m.is_empty() => m.clone(),
            _ => return Ok(None),
        };

        // 解密 api_key
        let api_key = match encrypted_key {
            Some(ref k) if !k.is_empty() => Some(crate::notifications::crypto::decrypt_token(k)?),
            _ => None,
        };

        Ok(Some(crate::notifications::models::AiProviderConfig {
            base_url,
            model,
            api_key,
        }))
    }

    pub fn reset_all_data(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        // 刪除所有資料（notification_history first due to FK constraints）
        conn.execute_batch(
            "DELETE FROM notification_history;
             DELETE FROM notification_rules;
             DELETE FROM notification_channels;
             DELETE FROM price_history;
             DELETE FROM view_subscriptions;
             DELETE FROM subscriptions;
             DELETE FROM views;
             DELETE FROM provider_settings;
             DELETE FROM app_settings;",
        )
        .map_err(|e| format!("Failed to delete all data: {}", e))?;

        // 重新插入預設 Views
        conn.execute_batch(
            "INSERT OR IGNORE INTO views (id, name, view_type, is_default) VALUES (1, 'All', 'asset', 1);
             INSERT OR IGNORE INTO views (id, name, view_type, is_default) VALUES (2, 'All', 'dex', 1);
             INSERT OR IGNORE INTO app_settings (key, value) VALUES ('api_port', '8080');
             INSERT OR IGNORE INTO app_settings (key, value) VALUES ('api_enabled', '0');"
        ).map_err(|e| format!("Failed to restore default data: {}", e))?;

        Ok(())
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
                .query_map([], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                })
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
}
