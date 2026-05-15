//! NotificationEngine — 推播通知引擎主邏輯
//!
//! 訂閱 Event Bus 的 PriceUpdate 事件，評估觸發條件，派發通知。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, RwLock};

use crate::db::DbPool;
use crate::events::AppEvent;
use crate::notifications::dispatcher;
use crate::notifications::evaluator;
use crate::notifications::models::{ConditionType, NotificationData, NotificationRule};

pub struct NotificationEngine {
    rules: Arc<RwLock<Vec<NotificationRule>>>,
    cooldowns: Arc<RwLock<HashMap<i64, Instant>>>,
    db: Arc<DbPool>,
    http_client: reqwest::Client,
}

impl NotificationEngine {
    pub fn new(db: Arc<DbPool>) -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            cooldowns: Arc::new(RwLock::new(HashMap::new())),
            db,
            http_client: reqwest::Client::new(),
        }
    }

    /// 啟動引擎，訂閱 Event Bus 並開始監聽
    pub fn start(&self, mut event_rx: broadcast::Receiver<AppEvent>) {
        let rules = self.rules.clone();
        let cooldowns = self.cooldowns.clone();
        let db = self.db.clone();
        let http_client = self.http_client.clone();

        tokio::spawn(async move {
            eprintln!("[NotificationEngine] 啟動，開始監聯事件");
            loop {
                match event_rx.recv().await {
                    Ok(AppEvent::PriceUpdate { data, .. }) => {
                        let rules_guard = rules.read().await;
                        for asset in &data {
                            let triggered = evaluator::evaluate_rules(&rules_guard, asset);
                            for rule in triggered {
                                // Check cooldown
                                let mut cd_guard = cooldowns.write().await;
                                if let Some(last_fired) = cd_guard.get(&rule.id) {
                                    if last_fired.elapsed().as_secs() < rule.cooldown_secs {
                                        continue;
                                    }
                                }
                                cd_guard.insert(rule.id, Instant::now());
                                drop(cd_guard);

                                // Build notification data and dispatch
                                let notif_data = NotificationData {
                                    symbol: asset.symbol.clone(),
                                    provider: asset.provider_id.clone(),
                                    price: asset.price,
                                    condition_type: rule.condition_type.clone(),
                                    threshold: rule.threshold,
                                    rule_name: rule.name.clone(),
                                    triggered_at: chrono::Utc::now(),
                                };
                                dispatcher::dispatch_notification(
                                    &db, &http_client, rule, &notif_data,
                                )
                                .await;
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        eprintln!("[NotificationEngine] 事件落後 {} 筆，繼續接收", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        eprintln!("[NotificationEngine] Event Bus 已關閉，引擎停止");
                        break;
                    }
                }
            }
        });
    }

    /// 重新載入規則（當 CRUD 操作後呼叫）
    pub async fn reload_rules(&self) {
        match self.load_rules_from_db() {
            Ok(new_rules) => {
                let mut rules_guard = self.rules.write().await;
                *rules_guard = new_rules;
                eprintln!("[NotificationEngine] 規則已重新載入，共 {} 條", rules_guard.len());
            }
            Err(e) => {
                eprintln!("[NotificationEngine] 載入規則失敗: {}", e);
            }
        }
    }

    /// 從資料庫載入規則並關聯 subscription 資訊
    fn load_rules_from_db(&self) -> Result<Vec<NotificationRule>, String> {
        let rule_rows = self.db.list_notification_rules()?;
        let subscriptions = self.db.list_all_subscriptions()?;

        let sub_map: HashMap<i64, (&str, &str)> = subscriptions
            .iter()
            .map(|s| (s.id, (s.selected_provider_id.as_str(), s.symbol.as_str())))
            .collect();

        let mut rules = Vec::new();
        for row in &rule_rows {
            if let Some(&(provider_id, symbol)) = sub_map.get(&row.subscription_id) {
                let condition_type = match ConditionType::from_str(&row.condition_type) {
                    Some(ct) => ct,
                    None => {
                        eprintln!(
                            "[NotificationEngine] 規則 {} 的條件類型無效: {}",
                            row.id, row.condition_type
                        );
                        continue;
                    }
                };
                let channel_ids: Vec<i64> =
                    serde_json::from_str(&row.channel_ids).unwrap_or_default();

                rules.push(NotificationRule {
                    id: row.id,
                    name: row.name.clone(),
                    subscription_id: row.subscription_id,
                    provider_id: provider_id.to_string(),
                    symbol: symbol.to_string(),
                    condition_type,
                    threshold: row.threshold,
                    channel_ids,
                    cooldown_secs: row.cooldown_secs as u64,
                    enabled: row.enabled,
                });
            } else {
                eprintln!(
                    "[NotificationEngine] 規則 {} 的訂閱 {} 不存在，跳過",
                    row.id, row.subscription_id
                );
            }
        }
        Ok(rules)
    }
}
