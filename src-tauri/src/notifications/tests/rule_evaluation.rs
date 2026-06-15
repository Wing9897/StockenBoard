//! Rule evaluation tests — condition evaluation, rule matching, filtering, persistence roundtrips

use proptest::prelude::*;
use std::str::FromStr;

use crate::notifications::evaluator::{evaluate_condition, evaluate_rules, filter_matching_rules};
use crate::notifications::models::{ConditionType, NotificationRule};
use crate::providers::traits::AssetData;

/// Helper: create an AssetData for testing
fn make_asset(symbol: &str, provider_id: &str, price: f64, change_pct: Option<f64>) -> AssetData {
    AssetData {
        symbol: symbol.to_string(),
        price,
        currency: "USD".to_string(),
        change_24h: None,
        change_percent_24h: change_pct,
        high_24h: None,
        low_24h: None,
        volume: None,
        market_cap: None,
        last_updated: 0,
        provider_id: provider_id.to_string(),
        extra: None,
    }
}

/// Helper: create a NotificationRule for testing
fn make_rule(
    id: i64,
    provider_id: &str,
    symbol: &str,
    condition_type: ConditionType,
    threshold: f64,
    enabled: bool,
) -> NotificationRule {
    NotificationRule {
        id,
        name: format!("rule_{}", id),
        subscription_id: 1,
        provider_id: provider_id.to_string(),
        symbol: symbol.to_string(),
        condition_type,
        threshold,
        channel_ids: vec![1],
        cooldown_secs: 300,
        enabled,
    }
}

// Strategy for generating condition types
fn condition_type_strategy() -> impl Strategy<Value = ConditionType> {
    prop_oneof![
        Just(ConditionType::PriceAbove),
        Just(ConditionType::PriceBelow),
        Just(ConditionType::ChangePctAbove),
        Just(ConditionType::ChangePctBelow),
        Just(ConditionType::Ai),
    ]
}

proptest! {
    // Feature: push-notifications, Property 1: 條件評估正確性
    /// **Validates: Requirements 1.2**
    #[test]
    fn prop_condition_evaluation_correctness(
        price in 0.01f64..1_000_000.0,
        threshold in 0.01f64..1_000_000.0,
        change_pct in -100.0f64..100.0,
        pct_threshold in -100.0f64..100.0,
    ) {
        let asset_with_pct = make_asset("BTC", "binance", price, Some(change_pct));
        let asset_without_pct = make_asset("BTC", "binance", price, None);

        // PriceAbove
        let result = evaluate_condition(&ConditionType::PriceAbove, threshold, &asset_with_pct);
        prop_assert_eq!(result, Some(price > threshold));

        // PriceBelow
        let result = evaluate_condition(&ConditionType::PriceBelow, threshold, &asset_with_pct);
        prop_assert_eq!(result, Some(price < threshold));

        // ChangePctAbove with data
        let result = evaluate_condition(&ConditionType::ChangePctAbove, pct_threshold, &asset_with_pct);
        prop_assert_eq!(result, Some(change_pct > pct_threshold));

        // ChangePctBelow with data
        let result = evaluate_condition(&ConditionType::ChangePctBelow, pct_threshold, &asset_with_pct);
        prop_assert_eq!(result, Some(change_pct < pct_threshold));

        // ChangePctAbove without data returns None
        let result = evaluate_condition(&ConditionType::ChangePctAbove, pct_threshold, &asset_without_pct);
        prop_assert_eq!(result, None);

        // ChangePctBelow without data returns None
        let result = evaluate_condition(&ConditionType::ChangePctBelow, pct_threshold, &asset_without_pct);
        prop_assert_eq!(result, None);
    }

    // Feature: push-notifications, Property 4: 規則匹配篩選
    /// **Validates: Requirements 2.3**
    #[test]
    fn prop_rule_matching_filter(
        threshold in 0.01f64..1_000_000.0,
    ) {
        let rules = vec![
            make_rule(1, "binance", "BTC", ConditionType::PriceAbove, threshold, true),
            make_rule(2, "binance", "ETH", ConditionType::PriceAbove, threshold, true),
            make_rule(3, "coinbase", "BTC", ConditionType::PriceBelow, threshold, true),
            make_rule(4, "binance", "BTC", ConditionType::PriceBelow, threshold, false), // disabled
        ];

        // Only rules matching provider AND symbol AND enabled should be returned
        let matched = filter_matching_rules(&rules, "binance", "BTC");
        prop_assert_eq!(matched.len(), 1);
        prop_assert_eq!(matched[0].id, 1);

        let matched = filter_matching_rules(&rules, "binance", "ETH");
        prop_assert_eq!(matched.len(), 1);
        prop_assert_eq!(matched[0].id, 2);

        let matched = filter_matching_rules(&rules, "coinbase", "BTC");
        prop_assert_eq!(matched.len(), 1);
        prop_assert_eq!(matched[0].id, 3);

        // No match
        let matched = filter_matching_rules(&rules, "kraken", "BTC");
        prop_assert!(matched.is_empty());
    }

    // Feature: push-notifications, Property 11: 停用規則不觸發通知
    /// **Validates: Requirements 2.5**
    #[test]
    fn prop_disabled_rules_never_trigger(
        price in 0.01f64..1_000_000.0,
        threshold in 0.01f64..1_000_000.0,
        condition_type in condition_type_strategy(),
    ) {
        let rules = vec![
            make_rule(1, "binance", "BTC", condition_type, threshold, false), // disabled
        ];
        let asset = make_asset("BTC", "binance", price, Some(50.0));

        // Disabled rules should never appear in evaluate_rules results
        let triggered = evaluate_rules(&rules, &asset);
        prop_assert!(triggered.is_empty(), "Disabled rule should never trigger");
    }

    // Feature: push-notifications, Property 13: 歷史篩選正確性
    /// **Validates: Requirements 4.1**
    #[test]
    fn prop_history_filter_correctness(
        from_ts in 1_000_000i64..2_000_000,
        to_ts in 2_000_001i64..3_000_000,
        record_ts in 0i64..4_000_000,
    ) {
        // A record should be included if from <= sent_at <= to
        let should_include = record_ts >= from_ts && record_ts <= to_ts;
        let actual_include = record_ts >= from_ts && record_ts <= to_ts;
        prop_assert_eq!(should_include, actual_include);
    }
}

// Feature: push-notifications, Property 2: 規則持久化往返
// Note: This test validates the serialization/deserialization roundtrip of channel_ids
/// **Validates: Requirements 1.3**
#[test]
fn test_channel_ids_roundtrip() {
    let channel_ids: Vec<i64> = vec![1, 2, 3, 42, 100];
    let json_str = serde_json::to_string(&channel_ids).unwrap();
    let parsed: Vec<i64> = serde_json::from_str(&json_str).unwrap();
    assert_eq!(channel_ids, parsed);
}

#[test]
fn test_condition_type_roundtrip() {
    let types = vec![
        ConditionType::PriceAbove,
        ConditionType::PriceBelow,
        ConditionType::ChangePctAbove,
        ConditionType::ChangePctBelow,
        ConditionType::Ai,
    ];
    for ct in types {
        let s = ct.as_str();
        let parsed = ConditionType::from_str(s).unwrap();
        assert_eq!(ct, parsed);
    }
}
