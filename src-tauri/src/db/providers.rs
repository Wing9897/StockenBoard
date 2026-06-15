use rusqlite::params;

use super::schema::{PollingProviderSetting, ProviderSettingsRow};
use super::DbPool;

impl DbPool {
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
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_provider_settings(
        &self,
        provider_id: &str,
    ) -> Result<Option<ProviderSettingsRow>, String> {
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

    // 引數對應 provider_settings 資料表欄位，刻意保持平面簽章
    #[allow(clippy::too_many_arguments)]
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
        .map_err(|e| format!("Failed to update provider settings: {}", e))?;
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

    /// 為 Polling 讀取所有 provider 設定
    pub fn read_polling_provider_settings(
        &self,
    ) -> Result<std::collections::HashMap<String, PollingProviderSetting>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT provider_id, api_key, api_secret, api_url, refresh_interval FROM provider_settings")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    (
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3).ok().flatten(),
                        row.get(4)?,
                    ),
                ))
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
