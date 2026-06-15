use rusqlite::params;

use super::schema::{ViewRow, ViewSubCount};
use super::DbPool;

impl DbPool {
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
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn create_view(&self, name: &str, view_type: &str) -> Result<i64, String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO views (name, view_type, is_default) VALUES (?1, ?2, 0)",
            params![name, view_type],
        )
        .map_err(|e| format!("Failed to create view: {}", e))?;
        Ok(conn.last_insert_rowid())
    }

    pub fn rename_view(&self, id: i64, name: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE views SET name = ?1 WHERE id = ?2",
            params![name, id],
        )
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
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_view_subscription_ids(&self, view_id: i64) -> Result<Vec<i64>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT subscription_id FROM view_subscriptions WHERE view_id = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([view_id], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
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
}
