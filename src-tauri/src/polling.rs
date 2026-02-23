use crate::providers::{create_provider, AssetData, DataProvider};
use crate::providers::traits::PROVIDER_INFO_MAP;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::{watch, RwLock};

/// 後端 polling tick — 每次 fetch 完成後推送給前端，讓倒計時精確同步
#[derive(Debug, Clone, Serialize)]
pub struct PollTick {
    pub provider_id: String,
    /// fetch 完成的時間戳 (ms since epoch)
    pub fetched_at: i64,
    /// 下次 fetch 前的 sleep 時間 (ms)
    pub interval_ms: u64,
}

/// 後端統一 polling 管理器
/// 讀取 DB subscriptions + provider_settings，按 provider 群組合併，
/// 定時 fetch，透過 Tauri event 推送給所有窗口
pub struct PollingManager {
    pub cache: Arc<RwLock<HashMap<String, AssetData>>>,
    /// 每個 provider 最近一次 poll-tick 快照（前端 F5 後可主動拉取）
    pub ticks: Arc<RwLock<HashMap<String, PollTick>>>,
    /// 前端目前可見的 subscription IDs（多窗口取聯集）
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
    refresh_interval: Option<i64>,
    enabled: bool,
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

    /// 通知 polling 重新載入 DB 配置
    pub fn reload(&self) {
        self.reload_tx.send_modify(|v| *v = v.wrapping_add(1));
    }

    /// 設定某個窗口目前可見的 subscription IDs
    /// window_id 用來區分多窗口，backend 取聯集
    pub async fn set_visible(&self, window_id: String, ids: HashSet<i64>) {
        let mut map = self.visible_ids.write().await;
        // 比較新舊值，無變化則跳過 reload
        if ids.is_empty() {
            if map.remove(&window_id).is_none() {
                return; // 本來就沒有，不需要 reload
            }
        } else {
            if let Some(existing) = map.get(&window_id) {
                if *existing == ids {
                    return; // 完全相同，不需要 reload
                }
            }
            map.insert(window_id, ids);
        }
        drop(map);
        self.reload_tx.send_modify(|v| *v = v.wrapping_add(1));
    }

    /// 啟動 polling 主循環（app setup 時呼叫一次）
    pub fn start(&self, app_handle: tauri::AppHandle, db_path: PathBuf) {
        let cache = self.cache.clone();
        let ticks = self.ticks.clone();
        let visible_ids = self.visible_ids.clone();
        let mut reload_rx = self.reload_tx.subscribe();
        let mut stop_rx = self.stop_tx.subscribe();

        tauri::async_runtime::spawn(async move {
            loop {
                // spawn_blocking 避免同步 DB I/O 阻塞 async runtime
                let db_path_clone = db_path.clone();
                let (vis_snapshot, has_windows): (HashSet<i64>, bool) = {
                    let map = visible_ids.read().await;
                    if map.is_empty() {
                        (HashSet::new(), false)
                    } else {
                        (map.values().flat_map(|s| s.iter().copied()).collect(), true)
                    }
                };
                // 如果有窗口但聯集為空（所有窗口都在空頁面），跳過 fetch
                if has_windows && vis_snapshot.is_empty() {
                    // 清理快取
                    cache.write().await.clear();
                    ticks.write().await.clear();
                    tokio::select! {
                        _ = reload_rx.changed() => continue,
                        _ = stop_rx.changed() => break,
                    }
                }
                let config = tokio::task::spawn_blocking(move || load_config(&db_path_clone, if has_windows { Some(&vis_snapshot) } else { None })).await;
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

                // 清理快取：移除已不存在的 subscription
                {
                    let valid: std::collections::HashSet<String> = groups
                        .iter()
                        .flat_map(|(pid, g)| g.symbols.iter().map(move |s| format!("{}:{}", pid, s)))
                        .collect();
                    cache.write().await.retain(|k, _| valid.contains(k));
                    // 清理不再活躍的 provider ticks
                    let active_pids: std::collections::HashSet<&String> = groups.keys().collect();
                    ticks.write().await.retain(|k, _| active_pids.contains(k));
                }

                if groups.is_empty() {
                    tokio::select! {
                        _ = reload_rx.changed() => continue,
                        _ = stop_rx.changed() => break,
                    }
                }

                // 為每個 provider group 啟動 polling task
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
                            // 通知前端：此 provider 剛完成 fetch，下次在 interval_ms 後
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

                // 等待 reload 或全局停止
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


/// 從 DB 讀取 subscriptions + provider_settings，組合成 polling groups
/// visible_ids: Some → 只 fetch 這些 subscription IDs；None → fetch 全部（啟動初期尚無窗口報告時）
fn load_config(
    db_path: &PathBuf,
    visible_ids: Option<&HashSet<i64>>,
) -> Result<(HashMap<String, PollingGroup>, HashMap<String, Arc<dyn DataProvider>>), String> {
    let conn = Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("開啟 DB 失敗: {}", e))?;

    // 讀取 subscriptions
    let subs: Vec<SubRecord> = {
        let mut stmt = conn
            .prepare("SELECT id, symbol, selected_provider_id FROM subscriptions")
            .map_err(|e| format!("查詢 subscriptions 失敗: {}", e))?;
        let rows = stmt.query_map([], |row| {
            Ok(SubRecord {
                id: row.get(0)?,
                symbol: row.get(1)?,
                provider_id: row.get(2)?,
            })
        })
        .map_err(|e| format!("讀取 subscriptions 失敗: {}", e))?;
        let all: Vec<SubRecord> = rows.filter_map(|r| r.ok()).collect();
        // 按 visible_ids 過濾
        match visible_ids {
            Some(ids) => all.into_iter().filter(|s| ids.contains(&s.id)).collect(),
            None => all,
        }
    };

    // 讀取 provider_settings
    let settings: HashMap<String, ProviderConfig> = {
        let mut stmt = conn
            .prepare(
                "SELECT provider_id, api_key, api_secret, refresh_interval, enabled FROM provider_settings",
            )
            .map_err(|e| format!("查詢 provider_settings 失敗: {}", e))?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                ProviderConfig {
                    api_key: row.get(1)?,
                    api_secret: row.get(2)?,
                    refresh_interval: row.get(3)?,
                    enabled: row.get::<_, i64>(4).unwrap_or(1) != 0,
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

        // 跳過被停用的 provider
        if config.map(|c| !c.enabled).unwrap_or(false) {
            continue;
        }

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
            if let Some(p) = create_provider(pid, api_key, api_secret) {
                provider_instances.insert(pid.clone(), p);
            }
        }
    }

    Ok((groups, provider_instances))
}
