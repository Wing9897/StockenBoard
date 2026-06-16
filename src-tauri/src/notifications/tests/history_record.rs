//! Property test for history record integrity.
//!
//! Feature: logo-management-and-local-notifications, Property 4: History record integrity
//! **Validates: Requirements 5.1, 5.2**
//!
//! Verifies that dispatching a notification through the local channel persists
//! a correct history record with matching rule_id, channel_id, status, price, and message.

use std::path::PathBuf;
use std::sync::Arc;

use proptest::prelude::*;

use crate::db::DbPool;
use crate::notifications::dispatcher::dispatch_notification;
use crate::notifications::models::{ConditionType, NotificationData, NotificationRule};

/// Helper: set up an in-memory DB with a local channel and a notification rule.
/// Returns (db, rule_id, local_channel_id).
fn setup_local_channel_db() -> (Arc<DbPool>, i64, i64) {
    let db = Arc::new(DbPool::open(&PathBuf::from(":memory:")).unwrap());

    // Create a subscription (required for rule FK)
    let sub_id = db
        .add_subscription(
            "asset", "BTC/USDT", None, "binance", "crypto", None, None, None,
        )
        .unwrap();

    // Create a local notification channel
    let channel_id = db
        .create_notification_channel("local", "Local", "{}")
        .unwrap();

    // Create a notification rule bound to the local channel
    let channel_ids_json = serde_json::to_string(&vec![channel_id]).unwrap();
    let rule_id = db
        .create_notification_rule(
            "Test Rule",
            sub_id,
            "price_above",
            100.0,
            &channel_ids_json,
            60,
            None,
        )
        .unwrap();

    (db, rule_id, channel_id)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    // Feature: logo-management-and-local-notifications, Property 4: History record integrity
    /// **Validates: Requirements 5.1, 5.2**
    #[test]
    fn prop_local_dispatch_persists_correct_history_record(
        symbol in "[A-Z]{2,6}/[A-Z]{2,6}",
        rule_name in "[a-zA-Z0-9 ]{1,30}",
        price in 0.01f64..1_000_000.0,
    ) {
        let (db, rule_id, channel_id) = setup_local_channel_db();

        let rule = NotificationRule {
            id: rule_id,
            name: rule_name.clone(),
            subscription_id: 1,
            provider_id: "binance".to_string(),
            symbol: symbol.clone(),
            condition_type: ConditionType::PriceAbove,
            threshold: 100.0,
            channel_ids: vec![channel_id],
            cooldown_secs: 60,
            enabled: true,
        };

        let data = NotificationData {
            symbol: symbol.clone(),
            provider: "binance".to_string(),
            price,
            condition_type: ConditionType::PriceAbove,
            threshold: 100.0,
            rule_name: rule_name.clone(),
            triggered_at: chrono::Utc::now(),
        };

        // Dispatch requires a tokio runtime
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let http_client = reqwest::Client::new();
        rt.block_on(dispatch_notification(&db, &http_client, &rule, &data));

        // Query the last history entry
        let history = db
            .query_notification_history(Some(rule_id), None, None, Some(1))
            .expect("Should query notification history");

        prop_assert!(!history.is_empty(), "History should have at least one record");

        let record = &history[0];
        prop_assert_eq!(record.rule_id, rule_id, "rule_id must match");
        prop_assert_eq!(record.channel_id, channel_id, "channel_id must match the local channel");
        prop_assert_eq!(&record.status, "success", "status must be 'success'");
        prop_assert_eq!(record.price, price, "price must match the dispatched price");
        prop_assert!(
            record.message.contains(&symbol),
            "message must contain symbol '{}', got: '{}'", symbol, record.message
        );
        prop_assert!(
            record.message.contains(&rule_name),
            "message must contain rule_name '{}', got: '{}'", rule_name, record.message
        );
        prop_assert!(record.error.is_none(), "error must be None for successful dispatch");
    }
}
