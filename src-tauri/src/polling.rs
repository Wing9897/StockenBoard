use crate::providers::traits::PROVIDER_INFO_MAP;
use crate::providers::{create_provider_with_url, AssetData, DataProvider};
use chrono::Timelike;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::{watch, RwLock};

#[derive(Debug, Clone, Serialize)]
pub struct PollTick {
    pub provider_id: String,
    pub fetched_at: i64,
    pub interval_ms: u64,
}

pub struct PollingManager {
    pub cache: Arc<RwLock<HashMap<String, AssetData>>>,
    pub ticks: Arc<RwLock<HashMap<String, PollTick>>>,
    visible_ids: Arc<RwLock<HashMap<String, HashSet<i64>>>>,
    unattended: Arc<RwLock<bool>>,
    reload_tx: watch::Sender<u64>,
    stop_tx: watch::Sender<bool>,
}

impl Clone for PollingManager {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            ticks: self.ticks.clone(),
            visible_ids: self.visible_ids.clone(),
            unattended: self.unattended.clone(),
            reload_tx: self.reload_tx.clone(),
            stop_tx: self.stop_tx.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct SubRecord {
    id: i64,
    symbol: String,
    provider_id: String,
    record_enabled: bool,
}

#[derive(Debug, Clone)]
struct ProviderConfig {
    api_key: Option<String>,
    api_secret: Option<String>,
    api_url: Option<String>,
    refresh_interval: Option<i64>,
}

#[derive(Debug)]
struct PollingGroup {
    symbols: Vec<String>,
    record_symbols: Vec<String>,
    interval_ms: u64,
}

impl PollingManager {
    pub fn new() -> Self {
        let (stop_tx, _) = watch::channel(false);
        let (reload_tx, _) = watch::channel(0u64);
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ticks: Arc::new(RwLock::new(HashMap::new())),
            visible_ids: Arc::new(RwLock::new(HashMap::new())),
            unattended: Arc::new(RwLock::new(false)),
            reload_tx,
            stop_tx,
        }
    }

    pub fn reload(&self) {
        self.reload_tx.send_modify(|v| *v = v.wrapping_add(1));
    }

    pub async fn set_visible(&self, window_id: String, ids: HashSet<i64>) {
        let mut map = self.visible_ids.write().await;
        if ids.is_empty() {
            if map.remove(&window_id).is_none() {
                return;
            }
        } else {
            if let Some(existing) = map.get(&window_id) {
                if *existing == ids {
                    return;
                }
            }
            map.insert(window_id, ids);
        }
        drop(map);
        self.reload_tx.send_modify(|v| *v = v.wrapping_add(1));
    }

    pub async fn set_unattended(&self, enabled: bool) {
        let mut flag = self.unattended.write().await;
        if *flag == enabled {
            return;
        }
        *flag = enabled;
        drop(flag);
        self.reload_tx.send_modify(|v| *v = v.wrapping_add(1));
    }

    pub async fn is_unattended(&self) -> bool {
        *self.unattended.read().await
    }

    pub fn start(&self, app_handle: tauri::AppHandle, db_path: PathBuf) {
        let cache = self.cache.clone();
        let ticks = self.ticks.clone();
        let visible_ids = self.visible_ids.clone();
        let unattended = self.unattended.clone();
        let mut reload_rx = self.reload_tx.subscribe();
        let mut stop_rx = self.stop_tx.subscribe();

        tauri::async_runtime::spawn(async move {
            loop {
                let db_path_clone = db_path.clone();
                let is_unattended = *unattended.read().await;

                // unattended 模式: 忽略 visible filter，poll 全部
                let (vis_snapshot, has_windows): (HashSet<i64>, bool) = if is_unattended {
                    (HashSet::new(), false) // None → load_config 不 filter
                } else {
                    let map = visible_ids.read().await;
                    if map.is_empty() {
                        (HashSet::new(), false)
                    } else {
                        (map.values().flat_map(|s| s.iter().copied()).collect(), true)
                    }
                };
                if !is_unattended && has_windows && vis_snapshot.is_empty() {
                    cache.write().await.clear();
                    ticks.write().await.clear();
                    tokio::select! {
                        _ = reload_rx.changed() => continue,
                        _ = stop_rx.changed() => break,
                    }
                }
                let config = tokio::task::spawn_blocking(move || {
                    load_config(
                        &db_path_clone,
                        if has_windows {
                            Some(&vis_snapshot)
                        } else {
                            None
                        },
                    )
                })
                .await;
                let (groups, providers) = match config {
                    Ok(Ok(v)) => v,
                    Ok(Err(e)) => {
                        eprintln!("[Polling] 讀取配置失敗: {}", e);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue;
                    }
                    Err(e) => {
                        eprintln!("[Polling] spawn_blocking 失敗: {}", e);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue;
                    }
                };

                {
                    let valid: HashSet<String> = groups
                        .iter()
                        .flat_map(|(pid, g)| {
                            g.symbols.iter().map(move |s| format!("{}:{}", pid, s))
                        })
                        .collect();
                    cache.write().await.retain(|k, _| valid.contains(k));
                    let active_pids: HashSet<&String> = groups.keys().collect();
                    ticks.write().await.retain(|k, _| active_pids.contains(k));
                }

                if groups.is_empty() {
                    tokio::select! {
                        _ = reload_rx.changed() => continue,
                        _ = stop_rx.changed() => break,
                    }
                }

                let (gen_stop_tx, _) = watch::channel(false);
                let mut handles = Vec::with_capacity(groups.len());

                for (provider_id, group) in &groups {
                    let provider = match providers.get(provider_id) {
                        Some(p) => p.clone(),
                        None => continue,
                    };
                    let symbols = group.symbols.clone();
                    let interval_ms = group.interval_ms;
                    let pid = provider_id.clone();
                    let cache = cache.clone();
                    let ticks = ticks.clone();
                    let app = app_handle.clone();
                    let mut gen_stop = gen_stop_tx.subscribe();
                    let record_enabled_ids: HashSet<String> =
                        group.record_symbols.iter().cloned().collect();
                    let db_for_history = db_path.clone();

                    handles.push(tokio::spawn(async move {
                        loop {
                            match provider.fetch_prices(&symbols).await {
                                Ok(results) => {
                                    {
                                        let mut c = cache.write().await;
                                        for d in &results {
                                            c.insert(format!("{}:{}", pid, d.symbol), d.clone());
                                        }
                                    }
                                    let _ = app.emit("price-update", &results);
                                    // 寫入 price_history（record_enabled 的訂閱）
                                    if !record_enabled_ids.is_empty() {
                                        let db_p = db_for_history.clone();
                                        let pid_c = pid.clone();
                                        let data = results.clone();
                                        let rids = record_enabled_ids.clone();
                                        let _ = tokio::task::spawn_blocking(move || {
                                            write_price_history(&db_p, &pid_c, &data, &rids);
                                        }).await;
                                    }
                                }
                                Err(e) => {
                                    eprintln!("[Polling] {} fetch 失敗: {}", pid, e);
                                    let payload: HashMap<String, String> = symbols
                                        .iter()
                                        .map(|s| (format!("{}:{}", pid, s), e.clone()))
                                        .collect();
                                    let _ = app.emit("price-error", &payload);
                                }
                            }
                            let tick = PollTick {
                                provider_id: pid.clone(),
                                fetched_at: chrono::Utc::now().timestamp_millis(),
                                interval_ms,
                            };
                            ticks.write().await.insert(pid.clone(), tick.clone());
                            let _ = app.emit("poll-tick", &tick);
                            tokio::select! {
                                _ = tokio::time::sleep(std::time::Duration::from_millis(interval_ms)) => {},
                                _ = gen_stop.changed() => break,
                            }
                        }
                    }));
                }

                tokio::select! {
                    _ = reload_rx.changed() => {},
                    _ = stop_rx.changed() => {
                        drop(gen_stop_tx);
                        for h in handles { h.abort(); }
                        break;
                    },
                }
                drop(gen_stop_tx);
                for h in handles {
                    h.abort();
                }
            }
        });
    }
}

/// 從 DB 讀取訂閱資訊並按 visible_ids 過濾
fn read_subscriptions(
    conn: &Connection,
    visible_ids: Option<&HashSet<i64>>,
) -> Result<Vec<SubRecord>, String> {
    let mut stmt = conn
        .prepare("SELECT id, sub_type, symbol, selected_provider_id, pool_address, token_from_address, token_to_address, record_enabled FROM subscriptions")
        .map_err(|e| format!("查詢 subscriptions 失敗: {}", e))?;
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
                let pool = pool_address.unwrap_or_default();
                let tf = token_from.unwrap_or_default();
                let tt = token_to.unwrap_or_default();
                format!("{}:{}:{}", pool, tf, tt)
            } else {
                symbol
            };

            Ok(SubRecord {
                id,
                symbol: final_symbol,
                provider_id,
                record_enabled: record_enabled != 0,
            })
        })
        .map_err(|e| format!("讀取 subscriptions 失敗: {}", e))?;
    let all: Vec<SubRecord> = rows.filter_map(|r| r.ok()).collect();
    Ok(match visible_ids {
        Some(ids) => all.into_iter().filter(|s| ids.contains(&s.id)).collect(),
        None => all,
    })
}

/// 從 DB 讀取 provider 設定
fn read_provider_settings_map(
    conn: &Connection,
) -> Result<HashMap<String, ProviderConfig>, String> {
    let mut stmt = conn
        .prepare("SELECT provider_id, api_key, api_secret, refresh_interval, api_url FROM provider_settings")
        .map_err(|e| format!("查詢 provider_settings 失敗: {}", e))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                ProviderConfig {
                    api_key: row.get(1)?,
                    api_secret: row.get(2)?,
                    refresh_interval: row.get(3)?,
                    api_url: row.get(4).ok().flatten(),
                },
            ))
        })
        .map_err(|e| format!("讀取 provider_settings 失敗: {}", e))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// 將訂閱資訊和 provider 設定組裝成 polling groups 和 provider instances
fn build_polling_groups(
    subs: &[SubRecord],
    settings: &HashMap<String, ProviderConfig>,
) -> (HashMap<String, PollingGroup>, HashMap<String, Arc<dyn DataProvider>>) {
    let info_map = &*PROVIDER_INFO_MAP;
    let mut groups: HashMap<String, PollingGroup> = HashMap::new();
    let mut provider_instances: HashMap<String, Arc<dyn DataProvider>> = HashMap::new();

    for sub in subs {
        let pid = &sub.provider_id;
        let config = settings.get(pid);

        let has_key = config
            .and_then(|c| c.api_key.as_ref())
            .map(|k| !k.is_empty())
            .unwrap_or(false);
        let default_interval = info_map
            .get(pid)
            .map(|i| {
                if has_key {
                    i.key_interval
                } else {
                    i.free_interval
                }
            })
            .unwrap_or(30000);
        let interval_ms = config
            .and_then(|c| c.refresh_interval)
            .unwrap_or(default_interval) as u64;

        let group = groups.entry(pid.clone()).or_insert_with(|| PollingGroup {
            symbols: Vec::new(),
            record_symbols: Vec::new(),
            interval_ms,
        });
        if !group.symbols.contains(&sub.symbol) {
            group.symbols.push(sub.symbol.clone());
        }
        if sub.record_enabled && !group.record_symbols.contains(&sub.symbol) {
            group.record_symbols.push(sub.symbol.clone());
        }

        if !provider_instances.contains_key(pid) {
            let api_key = config.and_then(|c| c.api_key.clone());
            let api_secret = config.and_then(|c| c.api_secret.clone());
            let api_url = config.and_then(|c| c.api_url.clone());
            if let Some(p) = create_provider_with_url(pid, api_key, api_secret, api_url) {
                provider_instances.insert(pid.clone(), p);
            }
        }
    }

    (groups, provider_instances)
}

/// 從統一 subscriptions 表讀取配置，組合成 polling groups
/// 對 DEX 類型 (sub_type='dex')，用 pool_address:token_from:token_to 組合 symbol
#[allow(clippy::type_complexity)]
fn load_config(
    db_path: &PathBuf,
    visible_ids: Option<&HashSet<i64>>,
) -> Result<
    (
        HashMap<String, PollingGroup>,
        HashMap<String, Arc<dyn DataProvider>>,
    ),
    String,
> {
    let conn = Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("開啟 DB 失敗: {}", e))?;

    let subs = read_subscriptions(&conn, visible_ids)?;
    let settings = read_provider_settings_map(&conn)?;
    drop(conn);

    Ok(build_polling_groups(&subs, &settings))
}

/// 寫入 price_history，5 秒去重
fn write_price_history(
    db_path: &PathBuf,
    provider_id: &str,
    data: &[AssetData],
    record_symbols: &HashSet<String>,
) {
    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[History] 開啟 DB 失敗: {}", e);
            return;
        }
    };
    let now = chrono::Utc::now().timestamp();
    let local_hour = chrono::Local::now().hour();
    for d in data {
        if !record_symbols.contains(&d.symbol) {
            continue;
        }
        // 查找 subscription_id + 紀錄時段
        let sub_row: Option<(i64, Option<i64>, Option<i64>)> = conn
            .prepare_cached("SELECT id, record_from_hour, record_to_hour FROM subscriptions WHERE symbol = ?1 AND selected_provider_id = ?2")
            .ok()
            .and_then(|mut stmt| stmt.query_row([&d.symbol, provider_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).ok());
        let (sub_id, sub_from, sub_to) = match sub_row {
            Some(r) => r,
            None => continue,
        };
        // 紀錄時段檢查：訂閱設定優先 > provider 設定 > 全天
        let (from_h, to_h) = if let (Some(from), Some(to)) = (sub_from, sub_to) {
            (from as u32, to as u32)
        } else {
            // 查 provider 層級時段
            let prov_hours: Option<(Option<i64>, Option<i64>)> = conn
                .prepare_cached("SELECT record_from_hour, record_to_hour FROM provider_settings WHERE provider_id = ?1")
                .ok()
                .and_then(|mut stmt| stmt.query_row([provider_id], |row| Ok((row.get(0)?, row.get(1)?))).ok());
            match prov_hours {
                Some((Some(pf), Some(pt))) => (pf as u32, pt as u32),
                _ => (0, 24), // 全天
            }
        };
        // 判斷本地時間是否在時段內（支援跨午夜，如 22-06）
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
            .and_then(|mut stmt| stmt.query_row(rusqlite::params![sub_id, now - 5], |_| Ok(true)).ok())
            .unwrap_or(false);
        if recent {
            continue;
        }
        // 從 extra 提取盤前/盤後價格
        let pre_price = d
            .extra
            .as_ref()
            .and_then(|e| e.get("pre_market_price"))
            .and_then(|v| v.as_f64());
        let post_price = d
            .extra
            .as_ref()
            .and_then(|e| e.get("post_market_price"))
            .and_then(|v| v.as_f64());
        let _ = conn.execute(
            "INSERT INTO price_history (subscription_id, provider_id, price, change_pct, volume, pre_price, post_price, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![sub_id, provider_id, d.price, d.change_percent_24h, d.volume, pre_price, post_price, now],
        );
    }
}
