use crate::db::DbPool;
use crate::events::AppEvent;
use crate::providers::registry::ProviderRegistry;
use crate::providers::traits::PROVIDER_INFO_MAP;
use crate::providers::AssetData;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, watch, RwLock};

#[derive(Debug, Clone, Serialize)]
pub struct PollTick {
    pub provider_id: String,
    pub fetched_at: i64,
    pub interval_ms: u64,
}

/// Tracks exponential backoff state for a provider that has consecutive failures.
/// Resets on the first successful fetch.
#[derive(Debug, Clone)]
pub struct BackoffState {
    pub consecutive_failures: u32,
    pub next_allowed_at: Instant,
}

/// Maximum backoff delay: 5 minutes (300,000ms)
const MAX_BACKOFF_MS: u64 = 300_000;

/// Base backoff interval in milliseconds
const BASE_BACKOFF_MS: u64 = 1_000;

pub struct PollingManager {
    pub cache: Arc<RwLock<HashMap<String, AssetData>>>,
    pub ticks: Arc<RwLock<HashMap<String, PollTick>>>,
    pub backoff: Arc<RwLock<HashMap<String, BackoffState>>>,
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
            backoff: self.backoff.clone(),
            visible_ids: self.visible_ids.clone(),
            unattended: self.unattended.clone(),
            reload_tx: self.reload_tx.clone(),
            stop_tx: self.stop_tx.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct ProviderConfig {
    api_key: Option<String>,
    refresh_interval: Option<i64>,
}

#[derive(Debug)]
struct PollingGroup {
    symbols: Vec<String>,
    record_symbols: Vec<String>,
    interval_ms: u64,
}

impl Default for PollingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PollingManager {
    pub fn new() -> Self {
        let (stop_tx, _) = watch::channel(false);
        let (reload_tx, _) = watch::channel(0u64);
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ticks: Arc::new(RwLock::new(HashMap::new())),
            backoff: Arc::new(RwLock::new(HashMap::new())),
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

    /// 啟動 Polling 主迴圈
    /// Polling 只負責取得數據並發送 AppEvent 到 event_bus，
    /// 不再直接寫 DB 或 emit 到前端（由 Forwarder 處理）
    pub fn start(
        &self,
        db: Arc<DbPool>,
        registry: Arc<ProviderRegistry>,
        event_bus: broadcast::Sender<AppEvent>,
    ) {
        let cache = self.cache.clone();
        let ticks = self.ticks.clone();
        let backoff = self.backoff.clone();
        let visible_ids = self.visible_ids.clone();
        let unattended = self.unattended.clone();
        let mut reload_rx = self.reload_tx.subscribe();
        let mut stop_rx = self.stop_tx.subscribe();

        tokio::spawn(async move {
            loop {
                let is_unattended = *unattended.read().await;

                let (vis_snapshot, has_windows): (HashSet<i64>, bool) = if is_unattended {
                    (HashSet::new(), false)
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

                let visible_ref = if has_windows {
                    Some(&vis_snapshot)
                } else {
                    None
                };
                let groups = match load_config(&db, visible_ref) {
                    Ok(g) => g,
                    Err(e) => {
                        eprintln!("[Polling] Failed to read config: {}", e);
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
                    let symbols = group.symbols.clone();
                    let interval_ms = group.interval_ms;
                    let pid = provider_id.clone();
                    let cache = cache.clone();
                    let ticks = ticks.clone();
                    let backoff = backoff.clone();
                    let mut gen_stop = gen_stop_tx.subscribe();
                    let record_symbols: Vec<String> = group.record_symbols.clone();
                    let db_clone = db.clone();
                    let reg = registry.clone();
                    let bus = event_bus.clone();

                    handles.push(tokio::spawn(async move {
                        loop {
                            // Check backoff: skip if provider is in backoff period
                            {
                                let backoff_map = backoff.read().await;
                                if let Some(state) = backoff_map.get(&pid) {
                                    if Instant::now() < state.next_allowed_at {
                                        eprintln!(
                                            "[Polling] Skipping {} — in backoff (failures: {}, level: {})",
                                            pid, state.consecutive_failures, state.consecutive_failures
                                        );
                                        drop(backoff_map);
                                        tokio::select! {
                                            _ = tokio::time::sleep(std::time::Duration::from_millis(interval_ms)) => {},
                                            _ = gen_stop.changed() => break,
                                        }
                                        continue;
                                    }
                                }
                            }

                            match reg.fetch_with_limit(&pid, &symbols, &db_clone).await {
                                Ok(results) => {
                                    // On success: reset backoff state for this provider
                                    {
                                        let mut backoff_map = backoff.write().await;
                                        backoff_map.remove(&pid);
                                    }
                                    // 更新本地快取（保持 get_cached_prices 功能）
                                    {
                                        let mut c = cache.write().await;
                                        for d in &results {
                                            c.insert(format!("{}:{}", pid, d.symbol), d.clone());
                                        }
                                    }
                                    // 發送 PriceUpdate 到 event bus
                                    let _ = bus.send(AppEvent::PriceUpdate {
                                        provider_id: pid.clone(),
                                        data: results,
                                        record_symbols: record_symbols.clone(),
                                    });
                                }
                                Err(e) => {
                                    eprintln!("[Polling] {} fetch failed: {}", pid, e);
                                    // On failure: increment failures, compute backoff delay
                                    {
                                        let mut backoff_map = backoff.write().await;
                                        let state = backoff_map
                                            .entry(pid.clone())
                                            .or_insert_with(|| BackoffState {
                                                consecutive_failures: 0,
                                                next_allowed_at: Instant::now(),
                                            });
                                        state.consecutive_failures += 1;
                                        let delay_ms = compute_backoff_delay(state.consecutive_failures);
                                        state.next_allowed_at = Instant::now()
                                            + std::time::Duration::from_millis(delay_ms);
                                    }
                                    let _ = bus.send(AppEvent::PriceError {
                                        provider_id: pid.clone(),
                                        symbols: symbols.clone(),
                                        error: e,
                                    });
                                }
                            }
                            // 發送 PollTick
                            let tick = PollTick {
                                provider_id: pid.clone(),
                                fetched_at: chrono::Utc::now().timestamp_millis(),
                                interval_ms,
                            };
                            ticks.write().await.insert(pid.clone(), tick.clone());
                            let _ = bus.send(AppEvent::PollTick {
                                provider_id: pid.clone(),
                                fetched_at: tick.fetched_at,
                                interval_ms,
                            });
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

/// Computes the backoff delay in milliseconds using exponential backoff.
/// Formula: min(BASE_BACKOFF_MS * 2^failures, MAX_BACKOFF_MS)
pub fn compute_backoff_delay(consecutive_failures: u32) -> u64 {
    let exp = 2u64.saturating_pow(consecutive_failures);
    let delay = BASE_BACKOFF_MS.saturating_mul(exp);
    delay.min(MAX_BACKOFF_MS)
}

/// 從 DbPool 讀取配置，組合成 polling groups
fn load_config(
    db: &Arc<DbPool>,
    visible_ids: Option<&HashSet<i64>>,
) -> Result<HashMap<String, PollingGroup>, String> {
    let all_subs = db.read_polling_subscriptions(visible_ids)?;
    let settings_map = db.read_polling_provider_settings()?;

    let info_map = &*PROVIDER_INFO_MAP;
    let mut groups: HashMap<String, PollingGroup> = HashMap::new();

    let mut configs: HashMap<String, ProviderConfig> = HashMap::new();
    for (pid, (api_key, _, _, refresh_interval)) in &settings_map {
        configs.insert(
            pid.clone(),
            ProviderConfig {
                api_key: api_key.clone(),
                refresh_interval: *refresh_interval,
            },
        );
    }

    for (_, symbol, provider_id, record_enabled) in &all_subs {
        let pid = provider_id;
        let config = configs.get(pid.as_str());

        let has_key = config
            .and_then(|c| c.api_key.as_ref())
            .map(|k| !k.is_empty())
            .unwrap_or(false);
        let default_interval = info_map
            .get(pid.as_str())
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
        if !group.symbols.contains(symbol) {
            group.symbols.push(symbol.clone());
        }
        if *record_enabled && !group.record_symbols.contains(symbol) {
            group.record_symbols.push(symbol.clone());
        }
    }

    Ok(groups)
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn test_backoff_delay_increases_exponentially() {
        // 1 failure: 1000 * 2^1 = 2000ms
        assert_eq!(compute_backoff_delay(1), 2_000);
        // 2 failures: 1000 * 2^2 = 4000ms
        assert_eq!(compute_backoff_delay(2), 4_000);
        // 3 failures: 1000 * 2^3 = 8000ms
        assert_eq!(compute_backoff_delay(3), 8_000);
        // 4 failures: 1000 * 2^4 = 16000ms
        assert_eq!(compute_backoff_delay(4), 16_000);
        // 5 failures: 1000 * 2^5 = 32000ms
        assert_eq!(compute_backoff_delay(5), 32_000);
    }

    #[test]
    fn test_backoff_delay_caps_at_max() {
        // At 9 failures: 1000 * 2^9 = 512_000 -> capped at 300_000
        assert_eq!(compute_backoff_delay(9), MAX_BACKOFF_MS);
        // At 10 failures: still capped
        assert_eq!(compute_backoff_delay(10), MAX_BACKOFF_MS);
        // Very large failure count: still capped
        assert_eq!(compute_backoff_delay(30), MAX_BACKOFF_MS);
    }

    #[test]
    fn test_backoff_delay_zero_failures() {
        // 0 failures: 1000 * 2^0 = 1000ms (base case)
        assert_eq!(compute_backoff_delay(0), 1_000);
    }

    #[tokio::test]
    async fn test_backoff_state_increment_on_failure() {
        let backoff = Arc::new(RwLock::new(HashMap::<String, BackoffState>::new()));
        let pid = "test_provider".to_string();

        // Simulate first failure
        {
            let mut map = backoff.write().await;
            let state = map.entry(pid.clone()).or_insert_with(|| BackoffState {
                consecutive_failures: 0,
                next_allowed_at: Instant::now(),
            });
            state.consecutive_failures += 1;
            let delay_ms = compute_backoff_delay(state.consecutive_failures);
            state.next_allowed_at = Instant::now() + Duration::from_millis(delay_ms);
        }

        {
            let map = backoff.read().await;
            let state = map.get(&pid).unwrap();
            assert_eq!(state.consecutive_failures, 1);
            assert!(state.next_allowed_at > Instant::now());
        }

        // Simulate second failure
        {
            let mut map = backoff.write().await;
            let state = map.get_mut(&pid).unwrap();
            state.consecutive_failures += 1;
            let delay_ms = compute_backoff_delay(state.consecutive_failures);
            state.next_allowed_at = Instant::now() + Duration::from_millis(delay_ms);
        }

        {
            let map = backoff.read().await;
            let state = map.get(&pid).unwrap();
            assert_eq!(state.consecutive_failures, 2);
        }
    }

    #[tokio::test]
    async fn test_backoff_state_reset_on_success() {
        let backoff = Arc::new(RwLock::new(HashMap::<String, BackoffState>::new()));
        let pid = "test_provider".to_string();

        // Add a backoff state (simulating failures)
        {
            let mut map = backoff.write().await;
            map.insert(
                pid.clone(),
                BackoffState {
                    consecutive_failures: 5,
                    next_allowed_at: Instant::now() + Duration::from_secs(60),
                },
            );
        }

        // On success: remove the backoff entry
        {
            let mut map = backoff.write().await;
            map.remove(&pid);
        }

        // Verify it was reset
        {
            let map = backoff.read().await;
            assert!(map.get(&pid).is_none());
        }
    }

    #[tokio::test]
    async fn test_backoff_skip_during_cooldown() {
        let backoff = Arc::new(RwLock::new(HashMap::<String, BackoffState>::new()));
        let pid = "test_provider".to_string();

        // Set backoff with a future next_allowed_at
        {
            let mut map = backoff.write().await;
            map.insert(
                pid.clone(),
                BackoffState {
                    consecutive_failures: 3,
                    next_allowed_at: Instant::now() + Duration::from_secs(60),
                },
            );
        }

        // Check if provider should be skipped
        let should_skip = {
            let map = backoff.read().await;
            if let Some(state) = map.get(&pid) {
                Instant::now() < state.next_allowed_at
            } else {
                false
            }
        };
        assert!(should_skip, "Provider should be skipped during backoff");
    }

    #[tokio::test]
    async fn test_backoff_allows_after_cooldown_expires() {
        let backoff = Arc::new(RwLock::new(HashMap::<String, BackoffState>::new()));
        let pid = "test_provider".to_string();

        // Set backoff with a past next_allowed_at (already expired)
        {
            let mut map = backoff.write().await;
            map.insert(
                pid.clone(),
                BackoffState {
                    consecutive_failures: 2,
                    next_allowed_at: Instant::now() - Duration::from_secs(1),
                },
            );
        }

        // Check if provider should be skipped
        let should_skip = {
            let map = backoff.read().await;
            if let Some(state) = map.get(&pid) {
                Instant::now() < state.next_allowed_at
            } else {
                false
            }
        };
        assert!(!should_skip, "Provider should NOT be skipped after backoff expires");
    }

    #[test]
    fn test_backoff_delay_progression() {
        // Verify the full progression from 1 failure to cap
        let expected: Vec<(u32, u64)> = vec![
            (1, 2_000),
            (2, 4_000),
            (3, 8_000),
            (4, 16_000),
            (5, 32_000),
            (6, 64_000),
            (7, 128_000),
            (8, 256_000),
            (9, 300_000), // capped
            (10, 300_000), // capped
        ];

        for (failures, expected_delay) in expected {
            assert_eq!(
                compute_backoff_delay(failures),
                expected_delay,
                "Mismatch at {} failures",
                failures
            );
        }
    }
}
