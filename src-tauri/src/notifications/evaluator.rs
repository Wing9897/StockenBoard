//! 條件評估邏輯
//!
//! 根據 ConditionType 和 AssetData 判斷是否觸發通知。

use crate::notifications::models::{ConditionType, NotificationRule};
use crate::providers::traits::AssetData;

/// 評估單一條件是否滿足
/// Returns Some(true) if the condition is triggered, Some(false) if not triggered,
/// or None if the required data is unavailable (e.g., change_percent_24h is None).
pub fn evaluate_condition(condition_type: &ConditionType, threshold: f64, asset: &AssetData) -> Option<bool> {
    match condition_type {
        ConditionType::PriceAbove => Some(asset.price > threshold),
        ConditionType::PriceBelow => Some(asset.price < threshold),
        ConditionType::ChangePctAbove => {
            asset.change_percent_24h.map(|pct| pct > threshold)
        }
        ConditionType::ChangePctBelow => {
            asset.change_percent_24h.map(|pct| pct < threshold)
        }
    }
}

/// 從一組規則中篩選出匹配指定 provider_id 和 symbol 的規則
/// 只回傳 enabled=true 且 provider_id 和 symbol 都匹配的規則
pub fn filter_matching_rules<'a>(
    rules: &'a [NotificationRule],
    provider_id: &str,
    symbol: &str,
) -> Vec<&'a NotificationRule> {
    rules
        .iter()
        .filter(|r| r.enabled && r.provider_id == provider_id && r.symbol == symbol)
        .collect()
}

/// 評估一組規則對一筆 AssetData，回傳所有被觸發的規則
/// 先篩選匹配的規則，再逐一評估條件，回傳條件滿足的規則
pub fn evaluate_rules<'a>(
    rules: &'a [NotificationRule],
    asset: &AssetData,
) -> Vec<&'a NotificationRule> {
    let matching = filter_matching_rules(rules, &asset.provider_id, &asset.symbol);
    matching
        .into_iter()
        .filter(|rule| {
            evaluate_condition(&rule.condition_type, rule.threshold, asset)
                .unwrap_or(false)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn make_rule(id: i64, provider_id: &str, symbol: &str, condition_type: ConditionType, threshold: f64, enabled: bool) -> NotificationRule {
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

    // === evaluate_condition tests ===

    #[test]
    fn test_price_above_triggered() {
        let asset = make_asset("BTC", "binance", 70000.0, None);
        let result = evaluate_condition(&ConditionType::PriceAbove, 65000.0, &asset);
        assert_eq!(result, Some(true));
    }

    #[test]
    fn test_price_above_not_triggered() {
        let asset = make_asset("BTC", "binance", 60000.0, None);
        let result = evaluate_condition(&ConditionType::PriceAbove, 65000.0, &asset);
        assert_eq!(result, Some(false));
    }

    #[test]
    fn test_price_above_equal_not_triggered() {
        let asset = make_asset("BTC", "binance", 65000.0, None);
        let result = evaluate_condition(&ConditionType::PriceAbove, 65000.0, &asset);
        assert_eq!(result, Some(false));
    }

    #[test]
    fn test_price_below_triggered() {
        let asset = make_asset("BTC", "binance", 60000.0, None);
        let result = evaluate_condition(&ConditionType::PriceBelow, 65000.0, &asset);
        assert_eq!(result, Some(true));
    }

    #[test]
    fn test_price_below_not_triggered() {
        let asset = make_asset("BTC", "binance", 70000.0, None);
        let result = evaluate_condition(&ConditionType::PriceBelow, 65000.0, &asset);
        assert_eq!(result, Some(false));
    }

    #[test]
    fn test_price_below_equal_not_triggered() {
        let asset = make_asset("BTC", "binance", 65000.0, None);
        let result = evaluate_condition(&ConditionType::PriceBelow, 65000.0, &asset);
        assert_eq!(result, Some(false));
    }

    #[test]
    fn test_change_pct_above_triggered() {
        let asset = make_asset("BTC", "binance", 70000.0, Some(5.5));
        let result = evaluate_condition(&ConditionType::ChangePctAbove, 5.0, &asset);
        assert_eq!(result, Some(true));
    }

    #[test]
    fn test_change_pct_above_not_triggered() {
        let asset = make_asset("BTC", "binance", 70000.0, Some(3.0));
        let result = evaluate_condition(&ConditionType::ChangePctAbove, 5.0, &asset);
        assert_eq!(result, Some(false));
    }

    #[test]
    fn test_change_pct_above_none_returns_none() {
        let asset = make_asset("BTC", "binance", 70000.0, None);
        let result = evaluate_condition(&ConditionType::ChangePctAbove, 5.0, &asset);
        assert_eq!(result, None);
    }

    #[test]
    fn test_change_pct_below_triggered() {
        let asset = make_asset("BTC", "binance", 70000.0, Some(-3.0));
        let result = evaluate_condition(&ConditionType::ChangePctBelow, -2.0, &asset);
        assert_eq!(result, Some(true));
    }

    #[test]
    fn test_change_pct_below_not_triggered() {
        let asset = make_asset("BTC", "binance", 70000.0, Some(1.0));
        let result = evaluate_condition(&ConditionType::ChangePctBelow, -2.0, &asset);
        assert_eq!(result, Some(false));
    }

    #[test]
    fn test_change_pct_below_none_returns_none() {
        let asset = make_asset("BTC", "binance", 70000.0, None);
        let result = evaluate_condition(&ConditionType::ChangePctBelow, -2.0, &asset);
        assert_eq!(result, None);
    }

    // === filter_matching_rules tests ===

    #[test]
    fn test_filter_matching_rules_basic() {
        let rules = vec![
            make_rule(1, "binance", "BTC", ConditionType::PriceAbove, 65000.0, true),
            make_rule(2, "binance", "ETH", ConditionType::PriceAbove, 3000.0, true),
            make_rule(3, "coinbase", "BTC", ConditionType::PriceBelow, 60000.0, true),
        ];
        let matched = filter_matching_rules(&rules, "binance", "BTC");
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].id, 1);
    }

    #[test]
    fn test_filter_matching_rules_excludes_disabled() {
        let rules = vec![
            make_rule(1, "binance", "BTC", ConditionType::PriceAbove, 65000.0, false),
            make_rule(2, "binance", "BTC", ConditionType::PriceBelow, 60000.0, true),
        ];
        let matched = filter_matching_rules(&rules, "binance", "BTC");
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].id, 2);
    }

    #[test]
    fn test_filter_matching_rules_empty_when_no_match() {
        let rules = vec![
            make_rule(1, "binance", "BTC", ConditionType::PriceAbove, 65000.0, true),
        ];
        let matched = filter_matching_rules(&rules, "coinbase", "ETH");
        assert!(matched.is_empty());
    }

    // === evaluate_rules tests ===

    #[test]
    fn test_evaluate_rules_triggers_matching() {
        let rules = vec![
            make_rule(1, "binance", "BTC", ConditionType::PriceAbove, 65000.0, true),
            make_rule(2, "binance", "BTC", ConditionType::PriceBelow, 60000.0, true),
            make_rule(3, "binance", "ETH", ConditionType::PriceAbove, 3000.0, true),
        ];
        let asset = make_asset("BTC", "binance", 70000.0, None);
        let triggered = evaluate_rules(&rules, &asset);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].id, 1);
    }

    #[test]
    fn test_evaluate_rules_skips_disabled() {
        let rules = vec![
            make_rule(1, "binance", "BTC", ConditionType::PriceAbove, 65000.0, false),
        ];
        let asset = make_asset("BTC", "binance", 70000.0, None);
        let triggered = evaluate_rules(&rules, &asset);
        assert!(triggered.is_empty());
    }

    #[test]
    fn test_evaluate_rules_change_pct_none_skipped() {
        let rules = vec![
            make_rule(1, "binance", "BTC", ConditionType::ChangePctAbove, 5.0, true),
        ];
        let asset = make_asset("BTC", "binance", 70000.0, None);
        let triggered = evaluate_rules(&rules, &asset);
        assert!(triggered.is_empty());
    }

    #[test]
    fn test_evaluate_rules_multiple_triggered() {
        let rules = vec![
            make_rule(1, "binance", "BTC", ConditionType::PriceAbove, 65000.0, true),
            make_rule(2, "binance", "BTC", ConditionType::ChangePctAbove, 3.0, true),
        ];
        let asset = make_asset("BTC", "binance", 70000.0, Some(5.0));
        let triggered = evaluate_rules(&rules, &asset);
        assert_eq!(triggered.len(), 2);
    }
}
