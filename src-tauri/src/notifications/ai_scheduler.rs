//! AI 排程器模組 — 管理 AI 規則的 tokio task 生命週期
//!
//! 每條已啟用的 AI 規則對應一個獨立的 tokio task，按 analysis_interval_secs
//! 週期執行 AI 評估。AiScheduler 負責啟動、停止、重啟這些 task。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use tokio::task::AbortHandle;

use crate::db::DbPool;
use crate::events::{AppEvent, NotificationTriggeredPayload};
use crate::notifications::ai_evaluator;
use crate::notifications::dispatcher;
use crate::notifications::global_cooldown::GlobalCooldown;
use crate::notifications::models::{
    AiConfig, AiProviderConfig, ConditionType, NotificationData, NotificationRule,
};

/// 判斷是否應該抑制觸發（處於冷卻期）
///
/// 如果 `last_trigger` 距今的時間小於 `cooldown_secs`，則回傳 `true`（應抑制）。
/// 如果 `last_trigger` 為 `None`（從未觸發過），則回傳 `false`（不抑制）。
pub fn should_suppress_trigger(last_trigger: Option<Instant>, cooldown_secs: u64) -> bool {
    match last_trigger {
        Some(last) => last.elapsed() < std::time::Duration::from_secs(cooldown_secs),
        None => false,
    }
}

/// AI 排程器，管理所有 AI 規則的定期評估 task
pub struct AiScheduler {
    /// 規則 ID → TaskHandle 的映射，用於追蹤和管理各規則的 task
    tasks: Arc<RwLock<HashMap<i64, TaskHandle>>>,
    /// 資料庫連線池
    db: Arc<DbPool>,
    /// HTTP client，用於呼叫 AI API
    http_client: reqwest::Client,
    /// Event Bus sender — 觸發後發布 NotificationTriggered 供前端側欄即時顯示。
    /// 以 Option 包裝，讓測試可用無事件匯流排的精簡建構子。
    event_bus: Option<broadcast::Sender<AppEvent>>,
    /// 全局通知冷卻期（跨規則共享的最小觸發間隔）
    global_cooldown: Option<Arc<GlobalCooldown>>,
}

/// 單條 AI 規則對應的 task 控制柄
struct TaskHandle {
    /// 用於中止 task 的 handle
    abort_handle: AbortHandle,
}

impl AiScheduler {
    /// 建立新的 AiScheduler 實例
    ///
    /// # Arguments
    /// * `db` - 資料庫連線池的 Arc 引用
    pub fn new(db: Arc<DbPool>) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            db,
            http_client: reqwest::Client::new(),
            event_bus: None,
            global_cooldown: None,
        }
    }

    /// 設定 Event Bus sender（建構後注入），供觸發時推送前端事件。
    pub fn with_event_bus(mut self, event_bus: broadcast::Sender<AppEvent>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    /// 設定全局冷卻期（建構後注入），供觸發前檢查。
    pub fn with_global_cooldown(mut self, global_cooldown: Arc<GlobalCooldown>) -> Self {
        self.global_cooldown = Some(global_cooldown);
        self
    }

    /// 啟動排程器，載入所有已啟用的 AI 規則並為每條規則啟動 task
    ///
    /// 應在應用程式啟動時呼叫一次。
    pub async fn start(&self) {
        // Step 1: Load all notification rules from DB
        let rules = match self.db.list_notification_rules() {
            Ok(rules) => rules,
            Err(e) => {
                eprintln!("[AiScheduler] Failed to load notification rules: {}", e);
                return;
            }
        };

        // Step 2: Load AI provider config
        let provider_config = match self.db.load_ai_provider_config() {
            Ok(Some(config)) => config,
            Ok(None) => {
                eprintln!("[AiScheduler] AI Provider not configured, all AI rules paused");
                return;
            }
            Err(e) => {
                eprintln!("[AiScheduler] Failed to load AI Provider config: {}", e);
                return;
            }
        };

        // Step 3: Filter AI rules that are enabled and have valid ai_config
        let mut tasks = self.tasks.write().await;
        for rule in &rules {
            if rule.condition_type != "ai" || !rule.enabled {
                continue;
            }

            let ai_config = match &rule.ai_config {
                Some(config_str) => match serde_json::from_str::<AiConfig>(config_str) {
                    Ok(config) => config,
                    Err(e) => {
                        eprintln!(
                            "[AiScheduler] rule_id={} failed to parse ai_config: {}",
                            rule.id, e
                        );
                        continue;
                    }
                },
                None => {
                    eprintln!(
                        "[AiScheduler] rule_id={} is AI rule but missing ai_config",
                        rule.id
                    );
                    continue;
                }
            };

            let channel_ids: Vec<i64> = serde_json::from_str(&rule.channel_ids).unwrap_or_default();

            let abort_handle = Self::spawn_rule_task(
                rule.id,
                rule.subscription_id,
                ai_config,
                provider_config.clone(),
                self.db.clone(),
                self.http_client.clone(),
                rule.cooldown_secs,
                channel_ids,
                self.event_bus.clone(),
                self.global_cooldown.clone(),
            );

            tasks.insert(rule.id, TaskHandle { abort_handle });
        }

        eprintln!(
            "[AiScheduler] Started, {} AI rules running",
            tasks.len()
        );
    }

    /// 新增或重啟某條規則的 task
    ///
    /// 若該規則已有執行中的 task，先中止再重新啟動。
    /// 用於規則建立或更新後同步排程狀態。
    ///
    /// # Arguments
    /// * `rule_id` - 要新增或重啟的規則 ID
    pub async fn upsert_rule(&self, rule_id: i64) {
        // Step 1: Remove existing task if any
        {
            let mut tasks = self.tasks.write().await;
            if let Some(handle) = tasks.remove(&rule_id) {
                handle.abort_handle.abort();
                eprintln!("[AiScheduler] rule_id={} stopped old task", rule_id);
            }
        }

        // Step 2: Load the rule from DB
        let rule = match self.db.get_notification_rule(rule_id) {
            Ok(Some(rule)) => rule,
            Ok(None) => {
                eprintln!("[AiScheduler] rule_id={} not found", rule_id);
                return;
            }
            Err(e) => {
                eprintln!("[AiScheduler] rule_id={} failed to load rule: {}", rule_id, e);
                return;
            }
        };

        // Step 3: Check if rule is AI type and enabled
        if rule.condition_type != "ai" || !rule.enabled {
            return;
        }

        // Step 4: Parse ai_config
        let ai_config = match &rule.ai_config {
            Some(config_str) => match serde_json::from_str::<AiConfig>(config_str) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!(
                        "[AiScheduler] rule_id={} failed to parse ai_config: {}",
                        rule_id, e
                    );
                    return;
                }
            },
            None => {
                eprintln!(
                    "[AiScheduler] rule_id={} is AI rule but missing ai_config",
                    rule_id
                );
                return;
            }
        };

        // Step 5: Load provider config
        let provider_config = match self.db.load_ai_provider_config() {
            Ok(Some(config)) => config,
            Ok(None) => {
                eprintln!(
                    "[AiScheduler] AI Provider not configured, rule_id={} cannot start",
                    rule_id
                );
                return;
            }
            Err(e) => {
                eprintln!(
                    "[AiScheduler] rule_id={} Failed to load AI Provider config: {}",
                    rule_id, e
                );
                return;
            }
        };

        // Step 6: Spawn new task
        let channel_ids: Vec<i64> = serde_json::from_str(&rule.channel_ids).unwrap_or_default();

        let abort_handle = Self::spawn_rule_task(
            rule.id,
            rule.subscription_id,
            ai_config,
            provider_config,
            self.db.clone(),
            self.http_client.clone(),
            rule.cooldown_secs,
            channel_ids,
            self.event_bus.clone(),
            self.global_cooldown.clone(),
        );

        let mut tasks = self.tasks.write().await;
        tasks.insert(rule_id, TaskHandle { abort_handle });
        eprintln!("[AiScheduler] rule_id={} started new task", rule_id);
    }

    /// 停止某條規則的 task
    ///
    /// 用於規則刪除或停用時停止定期評估。
    ///
    /// # Arguments
    /// * `rule_id` - 要停止的規則 ID
    pub async fn remove_rule(&self, rule_id: i64) {
        let mut tasks = self.tasks.write().await;
        if let Some(handle) = tasks.remove(&rule_id) {
            handle.abort_handle.abort();
            eprintln!("[AiScheduler] rule_id={} task stopped and removed", rule_id);
        }
    }

    /// 取得目前正在執行的 task 數量（用於測試驗證）
    #[cfg(test)]
    pub async fn task_count(&self) -> usize {
        self.tasks.read().await.len()
    }

    /// 重新載入所有 AI 規則
    ///
    /// 停止所有現有 task，重新從 DB 載入 enabled 的 AI 規則並啟動 task。
    /// 用於批量更新或設定變更後的完整重載。
    pub async fn reload(&self) {
        // Step 1: Abort all existing tasks
        {
            let mut tasks = self.tasks.write().await;
            for (rule_id, handle) in tasks.drain() {
                handle.abort_handle.abort();
                eprintln!("[AiScheduler] reload: rule_id={} task stopped", rule_id);
            }
        }

        // Step 2: Re-run start logic
        eprintln!("[AiScheduler] Reloading all AI rules...");
        self.start().await;
    }

    /// 為單條 AI 規則啟動一個 tokio task
    #[allow(clippy::too_many_arguments)]
    fn spawn_rule_task(
        rule_id: i64,
        subscription_id: i64,
        ai_config: AiConfig,
        provider_config: AiProviderConfig,
        db: Arc<DbPool>,
        http_client: reqwest::Client,
        cooldown_secs: i64,
        channel_ids: Vec<i64>,
        event_bus: Option<broadcast::Sender<AppEvent>>,
        global_cooldown: Option<Arc<GlobalCooldown>>,
    ) -> AbortHandle {
        let handle = tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(ai_config.analysis_interval_secs);
            let mut last_trigger_time: Option<Instant> = None;

            eprintln!(
                "[AiScheduler] rule_id={} task started, interval {}s, cooldown {}s",
                rule_id, ai_config.analysis_interval_secs, cooldown_secs
            );

            loop {
                // Step 1: Sleep for analysis_interval_secs
                tokio::time::sleep(interval).await;

                // Step 2: Call evaluate_ai_rule (waits for completion before next interval)
                let eval_result = ai_evaluator::evaluate_ai_rule(
                    &db,
                    &http_client,
                    rule_id,
                    subscription_id,
                    &ai_config,
                    &provider_config,
                )
                .await;

                match eval_result {
                    Ok(response) => {
                        if response.trigger {
                            // Step 3a: Check per-rule cooldown first (lightweight, no side effects)
                            let in_cooldown =
                                should_suppress_trigger(last_trigger_time, cooldown_secs as u64);

                            if in_cooldown {
                                eprintln!(
                                    "[AiScheduler] rule_id={} triggered but in cooldown, ignoring",
                                    rule_id
                                );
                                continue;
                            }

                            // Step 3b: Check global cooldown (atomically marks as triggered if passes)
                            if let Some(ref gc) = global_cooldown {
                                if !gc.check_and_trigger() {
                                    eprintln!(
                                        "[AiScheduler] rule_id={} global cooldown active, skipping",
                                        rule_id
                                    );
                                    continue;
                                }
                            }

                            // Step 3c: Not in cooldown — dispatch notification
                            // Get symbol from subscription
                            let symbol = match get_symbol_for_subscription(&db, subscription_id) {
                                Some(s) => s,
                                None => {
                                    eprintln!(
                                        "[AiScheduler] rule_id={} cannot get symbol for subscription_id={}",
                                        rule_id, subscription_id
                                    );
                                    continue;
                                }
                            };

                            // Build NotificationRule for dispatcher
                            let rule = NotificationRule {
                                id: rule_id,
                                name: format!("AI Rule #{}", rule_id),
                                subscription_id,
                                provider_id: String::new(),
                                symbol: symbol.clone(),
                                condition_type: ConditionType::Ai,
                                threshold: 0.0,
                                channel_ids: channel_ids.clone(),
                                cooldown_secs: cooldown_secs as u64,
                                enabled: true,
                            };

                            // Build NotificationData with symbol, AI reason, and trigger time
                            let triggered_at = chrono::Utc::now();
                            let notif_data = NotificationData {
                                symbol,
                                provider: String::new(),
                                price: 0.0,
                                condition_type: ConditionType::Ai,
                                threshold: 0.0,
                                rule_name: format!("[AI] {}", response.reason),
                                triggered_at,
                            };

                            dispatcher::dispatch_notification(
                                &db,
                                &http_client,
                                &rule,
                                &notif_data,
                            )
                            .await;

                            // 發布觸發事件供前端側欄即時顯示（AI 規則）
                            if let Some(bus) = &event_bus {
                                let _ = bus.send(AppEvent::NotificationTriggered(
                                    NotificationTriggeredPayload {
                                        rule_name: notif_data.rule_name.clone(),
                                        symbol: notif_data.symbol.clone(),
                                        provider: notif_data.provider.clone(),
                                        price: notif_data.price,
                                        condition_type: ConditionType::Ai.as_str().to_string(),
                                        threshold: 0.0,
                                        triggered_at: triggered_at.timestamp(),
                                        is_ai: true,
                                        ai_reason: Some(response.reason.clone()),
                                    },
                                ));
                            }

                            // Step 3c: Record trigger time for cooldown tracking
                            last_trigger_time = Some(Instant::now());

                            eprintln!(
                                "[AiScheduler] rule_id={} AI notification dispatched, reason: {}",
                                rule_id, response.reason
                            );
                        } else {
                            // Step 4: trigger = false, log and continue
                            eprintln!(
                                "[AiScheduler] rule_id={} AI decided not to trigger: {}",
                                rule_id, response.reason
                            );
                        }
                    }
                    Err(e) => {
                        // Step 4: Error — log and continue to next iteration
                        eprintln!("[AiScheduler] rule_id={} AI evaluation error: {}", rule_id, e);
                    }
                }
                // Step 5: Evaluation is complete; loop back to sleep for next interval
            }
        });

        handle.abort_handle()
    }
}

/// 從資料庫取得 subscription 的 symbol 名稱
fn get_symbol_for_subscription(db: &Arc<DbPool>, subscription_id: i64) -> Option<String> {
    let subscriptions = db.list_all_subscriptions().ok()?;
    subscriptions
        .into_iter()
        .find(|s| s.id == subscription_id)
        .map(|s| s.symbol)
}
