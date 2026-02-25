use crate::providers::{create_provider_with_url, AssetData, DataProvider};
use crate::providers::traits::PROVIDER_INFO_MAP;
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
    reload_tx: watch::Sender<u64>,
    stop_tx: watch::Sender<bool>,
}

#[derive(Debug, Clone)]
struct SubRecord {
    id: i64,
    symbol: String,
    provider_id: String,
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

    pub fn start(&self, app_handle: tauri::AppHandle, db_path: PathBuf) {
        let cache = self.cache.clone();
        let ticks = self.ticks.clone();
        let visible_ids = self.visible_ids.clone();
        let mut reload_rx = self.reload_tx.subscribe();
        let mut stop_rx = self.stop_tx.subscribe();

        tauri::async_runtime::spawn(async move {
            loop {
                let db_path_clone = db_path.clone();
                let (vis_snapshot, has_windows): (HashSet<i64>, bool) = {
                    let map = visible_ids.read().await;
                    if map.is_empty() {
                        (HashSet::new(), false)
                    } else {
                        (map.values().flat_map(|s| s.iter().copied()).collect(), true)
                    }
                };
                if has_windows && vis_snapshot.is_empty() {
                    cache.write().await.clear();
                    ticks.write().await.clear();
                    tokio::select! {
                        _ = reload_rx.changed() => continue,
                        _ = stop_rx.changed() => break,
                    }
                }
                let config = tokio::task::spawn_blocking(move || {
                    load_config(&db_path_clone, if has_windows { Some(&vis_snapshot) } else { None })
                }).await;
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
                        .flat_map(|(pid, g)| g.symbols.iter().map(move |s| format!("{}:{}", pid, s)))
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
                for h in handles { h.abort(); }
            }
        });
    }
}


/// 從統一 subscriptions 表讀取配置，組合成 polling groups
/// 對 DEX 類型 (sub_type='dex')，用 pool_address:token_from:token_to 組合 symbol
fn load_config(
    db_path: &PathBuf,
    visible_ids: Option<&HashSet<i64>>,
) -> Result<(HashMap<String, PollingGroup>, HashMap<String, Arc<dyn DataProvider>>), String> {
    let conn = Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("開啟 DB 失敗: {}", e))?;

    let subs: Vec<SubRecord> = {
        let mut stmt = conn
            .prepare("SELECT id, sub_type, symbol, selected_provider_id, pool_address, token_from_address, token_to_address FROM subscriptions")
            .map_err(|e| format!("查詢 subscriptions 失敗: {}", e))?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let sub_type: String = row.get(1)?;
            let symbol: String = row.get(2)?;
            let provider_id: String = row.get(3)?;
            let pool_address: Option<String> = row.get(4)?;
            let token_from: Option<String> = row.get(5)?;
            let token_to: Option<String> = row.get(6)?;

            let final_symbol = if sub_type == "dex" {
                let pool = pool_address.unwrap_or_default();
                let tf = token_from.unwrap_or_default();
                let tt = token_to.unwrap_or_default();
                format!("{}:{}:{}", pool, tf, tt)
            } else {
                symbol
            };

            Ok(SubRecord { id, symbol: final_symbol, provider_id })
        })
        .map_err(|e| format!("讀取 subscriptions 失敗: {}", e))?;
        let all: Vec<SubRecord> = rows.filter_map(|r| r.ok()).collect();
        match visible_ids {
            Some(ids) => all.into_iter().filter(|s| ids.contains(&s.id)).collect(),
            None => all,
        }
    };

    let settings: HashMap<String, ProviderConfig> = {
        let mut stmt = conn
            .prepare("SELECT provider_id, api_key, api_secret, refresh_interval, api_url FROM provider_settings")
            .map_err(|e| format!("查詢 provider_settings 失敗: {}", e))?;
        let rows = stmt.query_map([], |row| {
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
        rows.filter_map(|r| r.ok()).collect()
    };

    drop(conn);

    let info_map = &*PROVIDER_INFO_MAP;
    let mut groups: HashMap<String, PollingGroup> = HashMap::new();
    let mut provider_instances: HashMap<String, Arc<dyn DataProvider>> = HashMap::new();

    for sub in &subs {
        let pid = &sub.provider_id;
        let config = settings.get(pid);

        let has_key = config
            .and_then(|c| c.api_key.as_ref())
            .map(|k| !k.is_empty())
            .unwrap_or(false);
        let default_interval = info_map
            .get(pid)
            .map(|i| if has_key { i.key_interval } else { i.free_interval })
            .unwrap_or(30000);
        let interval_ms = config
            .and_then(|c| c.refresh_interval)
            .unwrap_or(default_interval) as u64;

        let group = groups.entry(pid.clone()).or_insert_with(|| PollingGroup {
            symbols: Vec::new(),
            interval_ms,
        });
        if !group.symbols.contains(&sub.symbol) {
            group.symbols.push(sub.symbol.clone());
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

    Ok((groups, provider_instances))
}
